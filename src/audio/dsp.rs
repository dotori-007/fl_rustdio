use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use rodio::Source;

pub const SPECTRUM_BANDS_COUNT: usize = 32;
pub const WAVEFORM_BUFFER_SIZE: usize = 512;

/// 실시간 DSP 이펙터 파라미터 (스레드 간 공유)
#[derive(Clone, Debug)]
pub struct DspParams {
    pub speed: f32,
    pub pitch_semitones: f32,
    pub vinyl_link: bool,
    pub bass_gain_db: f32,
    pub treble_gain_db: f32,
    pub reverb_pct: f32,
    pub volume: f32,
}

impl Default for DspParams {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pitch_semitones: 0.0,
            vinyl_link: false,
            bass_gain_db: 0.0,
            treble_gain_db: 0.0,
            reverb_pct: 0.0,
            volume: 1.0,
        }
    }
}

/// 실시간 오디오 비주얼라이저 상태 (UI 렌더링용)
pub struct VisualizerState {
    pub waveform: [f32; WAVEFORM_BUFFER_SIZE],
    pub waveform_idx: usize,
    pub spectrum_bars: [f32; SPECTRUM_BANDS_COUNT],
    pub peak_caps: [f32; SPECTRUM_BANDS_COUNT],
    pub rms_level: f32,
    pub is_active: bool,
    energy_accum: [f32; SPECTRUM_BANDS_COUNT],
    sample_counter: usize,
}

impl Default for VisualizerState {
    fn default() -> Self {
        Self {
            waveform: [0.0; WAVEFORM_BUFFER_SIZE],
            waveform_idx: 0,
            spectrum_bars: [0.0; SPECTRUM_BANDS_COUNT],
            peak_caps: [0.0; SPECTRUM_BANDS_COUNT],
            rms_level: 0.0,
            is_active: false,
            energy_accum: [0.0; SPECTRUM_BANDS_COUNT],
            sample_counter: 0,
        }
    }
}

impl VisualizerState {
    pub fn push_sample(&mut self, sample: f32) {
        self.waveform[self.waveform_idx] = sample;
        self.waveform_idx = (self.waveform_idx + 1) % WAVEFORM_BUFFER_SIZE;
        self.is_active = true;

        // 초경량 필터 뱅크 누적 (주파수 대역 추정)
        let abs_val = sample.abs();
        let idx = self.sample_counter % SPECTRUM_BANDS_COUNT;
        self.energy_accum[idx] += abs_val;
        self.sample_counter += 1;

        if self.sample_counter >= 256 {
            let mut sum_rms = 0.0_f32;
            for i in 0..SPECTRUM_BANDS_COUNT {
                let raw_val = (self.energy_accum[i] / 8.0).min(1.0);
                // 주파수 틸트 (고음역 가중치 보정)
                let tilt = 1.0 + (i as f32 / SPECTRUM_BANDS_COUNT as f32) * 1.8;
                let target = (raw_val * tilt).clamp(0.0, 1.0);

                // 스무딩 & 피크 캡
                self.spectrum_bars[i] = self.spectrum_bars[i] * 0.7 + target * 0.3;
                if self.spectrum_bars[i] > self.peak_caps[i] {
                    self.peak_caps[i] = self.spectrum_bars[i];
                } else {
                    self.peak_caps[i] = (self.peak_caps[i] - 0.02).max(0.0);
                }
                sum_rms += self.spectrum_bars[i];
                self.energy_accum[i] = 0.0;
            }
            self.rms_level = self.rms_level * 0.7 + (sum_rms / SPECTRUM_BANDS_COUNT as f32) * 0.3;
            self.sample_counter = 0;
        }
    }

    /// UI 프레임마다 호출하여 비활성 시 부드럽게 감쇠(Decay)
    #[allow(dead_code)]
    pub fn update_decay(&mut self) {
        for bar in self.spectrum_bars.iter_mut() {
            *bar = (*bar * 0.88 - 0.01).max(0.0);
        }
        for cap in self.peak_caps.iter_mut() {
            *cap = (*cap - 0.025).max(0.0);
        }
        self.rms_level = (self.rms_level * 0.88 - 0.01).max(0.0);
    }
}

/// 듀얼 딜레이 라인 실시간 피치 시프터 (독립 피치 시프트용)
pub struct DualDelayPitchShifter {
    buffer: Vec<f32>,
    write_idx: usize,
    phase1: f32,
    phase2: f32,
    window_size: f32,
}

impl DualDelayPitchShifter {
    pub fn new(sample_rate: f32) -> Self {
        let max_delay_samples = (sample_rate * 0.1).max(2048.0) as usize; // 최대 100ms 딜레이 버퍼
        Self {
            buffer: vec![0.0; max_delay_samples],
            write_idx: 0,
            phase1: 0.0,
            phase2: 0.5,
            window_size: (sample_rate * 0.040).max(512.0), // 40ms 윈도우
        }
    }

    pub fn process(&mut self, input: f32, semitones: f32) -> f32 {
        if semitones.abs() < 0.05 {
            return input;
        }

        let buf_len = self.buffer.len();
        self.buffer[self.write_idx] = input;

        // 반음에 따른 피치 레이트: rate = 1 - 2^(semitones / 12)
        let rate = 1.0 - 2.0_f32.powf(semitones / 12.0);
        let delta = rate / self.window_size;

        self.phase1 = (self.phase1 + delta).rem_euclid(1.0);
        self.phase2 = (self.phase2 + delta).rem_euclid(1.0);

        // 딜레이 오프셋
        let delay1 = self.phase1 * self.window_size;
        let delay2 = self.phase2 * self.window_size;

        let read_idx1 = (self.write_idx as f32 - delay1).rem_euclid(buf_len as f32) as usize;
        let read_idx2 = (self.write_idx as f32 - delay2).rem_euclid(buf_len as f32) as usize;

        // 크로스페이드 삼각/코사인 가중치
        let w1 = 0.5 * (1.0 - (2.0 * PI * self.phase1).cos());
        let w2 = 0.5 * (1.0 - (2.0 * PI * self.phase2).cos());

        let out = self.buffer[read_idx1] * w1 + self.buffer[read_idx2] * w2;
        self.write_idx = (self.write_idx + 1) % buf_len;

        out
    }
}
#[derive(Clone, Copy, Default)]
pub struct BiquadFilter {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    s1: f32,
    s2: f32,
}

impl BiquadFilter {
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        self.s1 = 0.0;
        self.s2 = 0.0;
    }

    /// Low-shelf 필터 (베이스 부스트)
    pub fn set_low_shelf(&mut self, sample_rate: f32, cutoff_hz: f32, gain_db: f32) {
        if gain_db.abs() < 0.01 {
            self.set_passthrough();
            return;
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * cutoff_hz / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * 2.0_f32.sqrt();

        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    /// High-shelf 필터 (고음역 에어 / 트레블 부스트)
    pub fn set_high_shelf(&mut self, sample_rate: f32, cutoff_hz: f32, gain_db: f32) {
        if gain_db.abs() < 0.01 {
            self.set_passthrough();
            return;
        }
        let a = 10.0_f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * cutoff_hz / sample_rate;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / 2.0 * 2.0_f32.sqrt();

        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha;
        let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
        let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha;

        let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * a.sqrt() * alpha);
        let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
        let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * a.sqrt() * alpha);

        self.b0 = b0 / a0;
        self.b1 = b1 / a0;
        self.b2 = b2 / a0;
        self.a1 = a1 / a0;
        self.a2 = a2 / a0;
    }

    pub fn set_passthrough(&mut self) {
        self.b0 = 1.0;
        self.b1 = 0.0;
        self.b2 = 0.0;
        self.a1 = 0.0;
        self.a2 = 0.0;
    }

    #[inline(always)]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.s1;
        self.s1 = self.b1 * x - self.a1 * y + self.s2;
        self.s2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// 간단한 Schroeder 리버브 필터 (Comb 4개 + Allpass 2개)
pub struct SimpleReverb {
    comb_buffers: [Vec<f32>; 4],
    comb_indices: [usize; 4],
    comb_gains: [f32; 4],
    allpass_buffers: [Vec<f32>; 2],
    allpass_indices: [usize; 2],
}

impl SimpleReverb {
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate / 44100.0;
        let comb_lengths = [
            (1116.0 * scale) as usize,
            (1188.0 * scale) as usize,
            (1277.0 * scale) as usize,
            (1356.0 * scale) as usize,
        ];
        let allpass_lengths = [(225.0 * scale) as usize, (341.0 * scale) as usize];

        Self {
            comb_buffers: [
                vec![0.0; comb_lengths[0].max(1)],
                vec![0.0; comb_lengths[1].max(1)],
                vec![0.0; comb_lengths[2].max(1)],
                vec![0.0; comb_lengths[3].max(1)],
            ],
            comb_indices: [0; 4],
            comb_gains: [0.773, 0.802, 0.753, 0.733],
            allpass_buffers: [
                vec![0.0; allpass_lengths[0].max(1)],
                vec![0.0; allpass_lengths[1].max(1)],
            ],
            allpass_indices: [0; 2],
        }
    }

    pub fn process(&mut self, input: f32, wet_pct: f32) -> f32 {
        if wet_pct <= 0.001 {
            return input;
        }

        let wet = (wet_pct / 100.0 * 0.45).clamp(0.0, 0.5);
        let dry = 1.0 - wet * 0.5;

        // 4개의 병렬 Comb 필터 합산
        let mut comb_sum = 0.0;
        for i in 0..4 {
            let buf = &mut self.comb_buffers[i];
            let idx = self.comb_indices[i];
            let out = buf[idx];
            buf[idx] = input + out * self.comb_gains[i];
            self.comb_indices[i] = (idx + 1) % buf.len();
            comb_sum += out;
        }
        comb_sum *= 0.25;

        // 2개의 직렬 Allpass 필터 통과
        let mut ap_out = comb_sum;
        let ap_gain = 0.5;
        for i in 0..2 {
            let buf = &mut self.allpass_buffers[i];
            let idx = self.allpass_indices[i];
            let delayed = buf[idx];
            let out = -ap_gain * ap_out + delayed;
            buf[idx] = ap_out + ap_gain * delayed;
            self.allpass_indices[i] = (idx + 1) % buf.len();
            ap_out = out;
        }

        // Dry / Wet 결합 및 소프트 리미팅
        let mixed = dry * input + wet * ap_out;
        mixed.clamp(-1.0, 1.0)
    }
}

/// rodio Source를 래핑하여 실시간 EQ, 리버브 및 비주얼라이저 버퍼를 처리하는 DSP 스트림
pub struct DspSource<I> {
    source: I,
    params: Arc<Mutex<DspParams>>,
    visualizer: Arc<Mutex<VisualizerState>>,
    sample_rate: u32,
    channels: u16,

    // DSP 필터 인스턴스 (채널별)
    pitch_shifters: Vec<DualDelayPitchShifter>,
    bass_filters: Vec<BiquadFilter>,
    treble_filters: Vec<BiquadFilter>,
    reverbs: Vec<SimpleReverb>,

    last_bass_gain: f32,
    last_treble_gain: f32,
    current_channel: usize,
}

impl<I> DspSource<I>
where
    I: Source<Item = f32>,
{
    pub fn new(
        source: I,
        params: Arc<Mutex<DspParams>>,
        visualizer: Arc<Mutex<VisualizerState>>,
    ) -> Self {
        let sample_rate = source.sample_rate();
        let channels = source.channels();
        let ch_count = channels as usize;

        let mut pitch_shifters = Vec::with_capacity(ch_count);
        let mut bass_filters = Vec::with_capacity(ch_count);
        let mut treble_filters = Vec::with_capacity(ch_count);
        let mut reverbs = Vec::with_capacity(ch_count);

        for _ in 0..ch_count {
            pitch_shifters.push(DualDelayPitchShifter::new(sample_rate as f32));

            let mut bf = BiquadFilter::default();
            bf.set_passthrough();
            bass_filters.push(bf);

            let mut tf = BiquadFilter::default();
            tf.set_passthrough();
            treble_filters.push(tf);

            reverbs.push(SimpleReverb::new(sample_rate as f32));
        }

        Self {
            source,
            params,
            visualizer,
            sample_rate,
            channels,
            pitch_shifters,
            bass_filters,
            treble_filters,
            reverbs,
            last_bass_gain: -999.0,
            last_treble_gain: -999.0,
            current_channel: 0,
        }
    }
}

impl<I> Iterator for DspSource<I>
where
    I: Source<Item = f32>,
{
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let raw_sample = self.source.next()?;
        let ch = self.current_channel;
        self.current_channel = (self.current_channel + 1) % (self.channels as usize);

        // 파라미터 확인 및 필터 업데이트
        let (speed, pitch_semi, vinyl, bass_gain, treble_gain, reverb_pct, vol) = if let Ok(p) = self.params.try_lock() {
            (p.speed, p.pitch_semitones, p.vinyl_link, p.bass_gain_db, p.treble_gain_db, p.reverb_pct, p.volume)
        } else {
            (1.0, 0.0, false, self.last_bass_gain, self.last_treble_gain, 0.0, 1.0)
        };

        if (bass_gain - self.last_bass_gain).abs() > 0.1 {
            self.last_bass_gain = bass_gain;
            for bf in self.bass_filters.iter_mut() {
                bf.set_low_shelf(self.sample_rate as f32, 160.0, bass_gain);
            }
        }

        if (treble_gain - self.last_treble_gain).abs() > 0.1 {
            self.last_treble_gain = treble_gain;
            for tf in self.treble_filters.iter_mut() {
                tf.set_high_shelf(self.sample_rate as f32, 7000.0, treble_gain);
            }
        }

        // 1. 피치 시프트 계산
        // - 바이닐 연동 모드(vinyl == true): 턴테이블/LP 가속이므로 속도에 따른 피치 상승을 자연스럽게 유지
        // - 독립 속도 모드(vinyl == false): sink.set_speed()로 인해 상승한 피치(+12*log2(speed))를 
        //   피치 시프터가 역보상(-12*log2(speed))하여 배속 조절 시에도 원곡 피치를 완벽하게 보존!
        let target_pitch = if vinyl {
            pitch_semi
        } else {
            let speed_factor = speed.clamp(0.25, 4.0);
            let speed_pitch_offset = 12.0 * speed_factor.log2();
            pitch_semi - speed_pitch_offset
        };

        let mut sample = if target_pitch.abs() > 0.02 {
            self.pitch_shifters[ch].process(raw_sample, target_pitch)
        } else {
            raw_sample
        };

        // 2. 베이스 부스트 필터 적용
        sample = self.bass_filters[ch].process(sample);

        // 3. 고음역 에어 / 트레블 부스트 필터 적용
        sample = self.treble_filters[ch].process(sample);

        // 4. 리버브 필터 적용
        sample = self.reverbs[ch].process(sample, reverb_pct);

        // 5. 볼륨 조절
        sample = (sample * vol).clamp(-1.0, 1.0);

        // 5. 비주얼라이저 버퍼에 첫 번째 채널 샘플 전송
        if ch == 0 {
            if let Ok(mut viz) = self.visualizer.try_lock() {
                viz.push_sample(sample);
            }
        }

        Some(sample)
    }
}

impl<I> Source for DspSource<I>
where
    I: Source<Item = f32>,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}
