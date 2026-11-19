use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source};
use super::dsp::{DspParams, DspSource, VisualizerState};

pub struct AudioPlayer {
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    current_file: Option<PathBuf>,
    pub is_playing: bool,
    pub volume: f32,
    pub duration: Duration,
    playback_start: Option<Instant>,
    accumulated_time: Duration,
    pub dsp_params: Arc<Mutex<DspParams>>,
    pub visualizer: Arc<Mutex<VisualizerState>>,
    pub waveform_peaks: Vec<f32>,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (stream, handle) = match OutputStream::try_default() {
            Ok((s, h)) => (Some(s), Some(h)),
            Err(e) => {
                eprintln!("오디오 출력 장치 초기화 실패: {e}");
                (None, None)
            }
        };

        Self {
            _stream: stream,
            stream_handle: handle,
            sink: None,
            current_file: None,
            is_playing: false,
            volume: 1.0,
            duration: Duration::ZERO,
            playback_start: None,
            accumulated_time: Duration::ZERO,
            dsp_params: Arc::new(Mutex::new(DspParams::default())),
            visualizer: Arc::new(Mutex::new(VisualizerState::default())),
            waveform_peaks: Vec::new(),
        }
    }

    pub fn update_dsp_params(&mut self, params: &DspParams) {
        let old_speed = self.dsp_params.lock().map(|p| p.speed).unwrap_or(1.0);
        let new_speed = params.speed.clamp(0.25, 4.0);

        if (old_speed - new_speed).abs() > 0.005 && self.is_playing {
            // 속도가 변경될 때 기존 배속으로 누적 재생 시간 정산
            if let Some(start) = self.playback_start.take() {
                self.accumulated_time += start.elapsed().mul_f32(old_speed);
                self.playback_start = Some(Instant::now());
            }
        }

        if let Ok(mut p) = self.dsp_params.lock() {
            *p = params.clone();
            p.volume = self.volume;
        }

        // 속도가 실제로 달라졌을 때만 Sink::set_speed 호출 (불필요한 락 경합 및 오디오 언더런 방지)
        if (old_speed - new_speed).abs() > 0.005 {
            if let Some(ref sink) = self.sink {
                sink.set_speed(new_speed);
            }
        }
    }

    #[allow(dead_code)]
    pub fn load_file(&mut self, path: &Path, known_duration_secs: f64) -> Result<(), String> {
        self.load_file_with_peaks(path, known_duration_secs, Vec::new())
    }

    pub fn load_file_with_peaks(&mut self, path: &Path, known_duration_secs: f64, peaks: Vec<f32>) -> Result<(), String> {
        self.stop();

        if let Some(ref handle) = self.stream_handle {
            let sink = Sink::try_new(handle).map_err(|e| format!("Sink 생성 실패: {e}"))?;
            sink.set_volume(self.volume);
            self.sink = Some(sink);
            self.current_file = Some(path.to_path_buf());
            self.duration = Duration::from_secs_f64(known_duration_secs.max(0.0));
            self.accumulated_time = Duration::ZERO;
            self.playback_start = None;
            self.is_playing = false;

            // 이미 워커 스레드에서 추출된 피크를 즉시 대입 (메인 스레드 블로킹 및 오디오 끊김 방지)
            if !peaks.is_empty() {
                self.waveform_peaks = peaks;
            } else {
                self.waveform_peaks = extract_waveform_peaks(path, 220);
            }

            Ok(())
        } else {
            Err("오디오 출력 장치를 사용할 수 없습니다.".to_string())
        }
    }

    pub fn play(&mut self) -> Result<(), String> {
        let file_path = match self.current_file.as_ref() {
            Some(p) => p.clone(),
            None => return Err("재생할 파일이 없습니다.".to_string()),
        };

        if let Some(ref sink) = self.sink {
            if sink.is_paused() {
                sink.play();
                self.is_playing = true;
                self.playback_start = Some(Instant::now());
                return Ok(());
            }

            if sink.empty() {
                let file = File::open(&file_path).map_err(|e| format!("파일 열기 실패: {e}"))?;
                let source = Decoder::new(BufReader::new(file)).map_err(|e| format!("오디오 디코딩 실패: {e}"))?;
                let f32_source = source.convert_samples::<f32>();
                let dsp_source = DspSource::new(f32_source, self.dsp_params.clone(), self.visualizer.clone());
                sink.append(dsp_source);

                let speed = self.dsp_params.lock().map(|p| p.speed).unwrap_or(1.0);
                sink.set_speed(speed.clamp(0.25, 4.0));
                sink.play();
                self.is_playing = true;
                self.playback_start = Some(Instant::now());
                return Ok(());
            }
        }

        Ok(())
    }

    pub fn pause(&mut self) {
        if let Some(ref sink) = self.sink {
            if self.is_playing {
                sink.pause();
                self.is_playing = false;
                if let Some(start) = self.playback_start.take() {
                    let speed = self.dsp_params.lock().map(|p| p.speed).unwrap_or(1.0).clamp(0.25, 4.0);
                    self.accumulated_time += start.elapsed().mul_f32(speed);
                }
            }
        }
    }

    pub fn toggle_play(&mut self) -> Result<(), String> {
        if self.is_playing {
            self.pause();
            Ok(())
        } else {
            self.play()
        }
    }

    pub fn stop(&mut self) {
        if let Some(ref sink) = self.sink {
            sink.stop();
        }
        self.sink = None;
        self.is_playing = false;
        self.playback_start = None;
        self.accumulated_time = Duration::ZERO;

        if let Some(ref handle) = self.stream_handle {
            if let Ok(new_sink) = Sink::try_new(handle) {
                new_sink.set_volume(self.volume);
                self.sink = Some(new_sink);
            }
        }
    }

    pub fn set_volume(&mut self, vol: f32) {
        self.volume = vol.clamp(0.0, 2.0);
        if let Ok(mut p) = self.dsp_params.lock() {
            p.volume = self.volume;
        }
        if let Some(ref sink) = self.sink {
            sink.set_volume(self.volume);
        }
    }

    pub fn current_position(&self) -> Duration {
        let mut pos = self.accumulated_time;
        if self.is_playing {
            if let Some(start) = self.playback_start {
                let speed = self.dsp_params.lock().map(|p| p.speed).unwrap_or(1.0).clamp(0.25, 4.0);
                pos += start.elapsed().mul_f32(speed);
            }
        }
        if self.duration > Duration::ZERO && pos > self.duration {
            self.duration
        } else {
            pos
        }
    }

    pub fn progress_ratio(&self) -> f32 {
        if self.duration.as_secs_f32() <= 0.0 {
            0.0
        } else {
            (self.current_position().as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0)
        }
    }

    pub fn seek_to_ratio(&mut self, ratio: f32) -> Result<(), String> {
        let target_secs = self.duration.as_secs_f64() * (ratio.clamp(0.0, 1.0) as f64);
        let target_pos = Duration::from_secs_f64(target_secs);

        let file_path = match self.current_file.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(()),
        };

        if let Some(ref sink) = self.sink {
            sink.stop();
        }

        if let Some(ref handle) = self.stream_handle {
            let sink = Sink::try_new(handle).map_err(|e| format!("Sink 생성 실패: {e}"))?;
            sink.set_volume(self.volume);

            let file = File::open(&file_path).map_err(|e| format!("파일 열기 실패: {e}"))?;
            let source = Decoder::new(BufReader::new(file)).map_err(|e| format!("오디오 디코딩 실패: {e}"))?;

            let skipped = source.skip_duration(target_pos);
            let f32_source = skipped.convert_samples::<f32>();
            let dsp_source = DspSource::new(f32_source, self.dsp_params.clone(), self.visualizer.clone());
            sink.append(dsp_source);

            let speed = self.dsp_params.lock().map(|p| p.speed).unwrap_or(1.0);
            sink.set_speed(speed.clamp(0.25, 4.0));

            self.accumulated_time = target_pos;
            if self.is_playing {
                sink.play();
                self.playback_start = Some(Instant::now());
            } else {
                sink.pause();
                self.playback_start = None;
            }

            self.sink = Some(sink);
        }

        Ok(())
    }
}

/// 오디오 파일에서 전체 재생 구간의 진폭 피크(Waveform Peaks)를 스트리밍 추출
pub fn extract_waveform_peaks(path: &Path, num_peaks: usize) -> Vec<f32> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = BufReader::new(file);
    let decoder = match Decoder::new(reader) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    let mut source = decoder.convert_samples::<f32>();
    let total_dur = source.total_duration().unwrap_or(Duration::from_secs(180));
    let s_rate = source.sample_rate();
    let channels = source.channels();

    let total_samples = (total_dur.as_secs_f64() * s_rate as f64 * channels as f64) as usize;
    let chunk_size = (total_samples / num_peaks).max(64);

    let mut peaks = Vec::with_capacity(num_peaks);
    let mut current_max = 0.0_f32;
    let mut count = 0;

    while let Some(sample) = source.next() {
        let abs_val = sample.abs();
        if abs_val > current_max {
            current_max = abs_val;
        }
        count += 1;

        if count >= chunk_size {
            peaks.push(current_max);
            current_max = 0.0;
            count = 0;
            if peaks.len() >= num_peaks {
                break;
            }
        }
    }

    if count > 0 && peaks.len() < num_peaks {
        peaks.push(current_max);
    }

    // 최소 피크가 부족한 경우 보정
    while peaks.len() < num_peaks && !peaks.is_empty() {
        let last = *peaks.last().unwrap_or(&0.1);
        peaks.push(last * 0.9);
    }

    // 0.05 ~ 1.0 범위로 정규화 (소리가 너무 작게 렌더링되지 않도록)
    let global_max = peaks.iter().cloned().fold(0.0_f32, f32::max);
    if global_max > 0.01 {
        for p in peaks.iter_mut() {
            *p = (*p / global_max).clamp(0.06, 1.0);
        }
    }

    peaks
}
