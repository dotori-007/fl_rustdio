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
    current_delta: f32,
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
            current_delta: 0.0,
        }
    }

    pub fn process(&mut self, input: f32, semitones: f32) -> f32 {
        let input = if input.is_finite() { input } else { 0.0 };
        let buf_len = self.buffer.len();
        if buf_len < 2 {
            return input;
        }

        // 항상 입력 샘플을 순환 버퍼에 지속 기록하여 부드러운 오디오 스트림 유지
        let cur_write = self.write_idx;
        self.buffer[cur_write] = input;
        self.write_idx = (cur_write + 1) % buf_len;

        // 반음에 따른 피치 레이트: rate = 1 - 2^(semitones / 12)
        let semitones_clamped = if semitones.is_finite() {
            semitones.clamp(-24.0, 24.0)
        } else {
            0.0
        };

        let target_delta = if semitones_clamped.abs() < 0.02 {
            0.0
        } else {
            let rate = 1.0 - 2.0_f32.powf(semitones_clamped / 12.0);
            rate / self.window_size
        };

        // 파라미터 글라이드 스무딩 (급격한 슬라이더 조작 시 틱/팝 방지)
        self.current_delta = self.current_delta * 0.95 + target_delta * 0.05;
        let delta = self.current_delta;

        // 피치 변화가 없을 때는 원본 샘플 반환 (무손실 패스스루)
        if delta.abs() < 1e-6 {
            return input;
        }

        self.phase1 = (self.phase1 + delta).rem_euclid(1.0);
        self.phase2 = (self.phase2 + delta).rem_euclid(1.0);

        if !self.phase1.is_finite() { self.phase1 = 0.0; }
        if !self.phase2.is_finite() { self.phase2 = 0.5; }

        // 딜레이 오프셋 (샘플 단위, 안전한 범위로 클램프)
        let max_safe_delay = (buf_len - 2) as f32;
        let delay1 = (self.phase1 * self.window_size).clamp(0.0, max_safe_delay);
        let delay2 = (self.phase2 * self.window_size).clamp(0.0, max_safe_delay);

        // 100% 패닉 방지: 정수 모듈로 + 선형 보간 안전 읽기
        let sample1 = self.read_interpolated(cur_write, delay1, buf_len);
        let sample2 = self.read_interpolated(cur_write, delay2, buf_len);

        // 부드러운 해닝(Hanning) 크로스페이드 가중치
        let w1 = 0.5 * (1.0 - (2.0 * PI * self.phase1).cos());
        let w2 = 0.5 * (1.0 - (2.0 * PI * self.phase2).cos());

        let out = sample1 * w1 + sample2 * w2;
        if out.is_finite() {
            out
        } else {
            input
        }
    }

    #[inline(always)]
    fn read_interpolated(&self, write_pos: usize, delay: f32, buf_len: usize) -> f32 {
        let delay_int = delay.floor() as usize;
        let frac = delay - delay.floor();

        // write_pos에서 delay_int만큼 과거의 인덱스 (정수 모듈로 연산으로 범위 초과 원천 차단)
        let idx0 = (write_pos + buf_len - (delay_int % buf_len)) % buf_len;
        let idx1 = (idx0 + buf_len - 1) % buf_len;

        self.buffer[idx0] * (1.0 - frac) + self.buffer[idx1] * frac
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
        let x = if x.is_finite() { x } else { 0.0 };
        let y = self.b0 * x + self.s1;
        self.s1 = self.b1 * x - self.a1 * y + self.s2;
        self.s2 = self.b2 * x - self.a2 * y;

        // x86 CPU 마이크로코드 스톨 방지: Denormal(Subnormal, <1e-15) 자동 0.0 플러시(FTZ)
        if self.s1.abs() < 1e-15 {
            self.s1 = 0.0;
        }
        if self.s2.abs() < 1e-15 {
            self.s2 = 0.0;
        }

        // 발산 방지 및 비정상 수치(NaN, Inf) 방어
        if !y.is_finite() || !self.s1.is_finite() || !self.s2.is_finite() {
            self.s1 = 0.0;
            self.s2 = 0.0;
            return x;
        }

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
        let input = if input.is_finite() { input } else { 0.0 };

        let wet = (wet_pct / 100.0 * 0.45).clamp(0.0, 0.5);
        let dry = 1.0 - wet * 0.5;

        // 4개의 병렬 Comb 필터 합산
        let mut comb_sum = 0.0;
        for i in 0..4 {
            let buf = &mut self.comb_buffers[i];
            let idx = self.comb_indices[i];
            let out = buf[idx];
            let next_val = input + out * self.comb_gains[i];
            // Denormal/Subnormal 및 비정상 수치 플러시
            buf[idx] = if next_val.abs() > 1e-15 && next_val.is_finite() {
                next_val
            } else {
                0.0
            };
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
            let next_buf = ap_out + ap_gain * delayed;
            // Denormal/Subnormal 및 비정상 수치 플러시
            buf[idx] = if next_buf.abs() > 1e-15 && next_buf.is_finite() {
                next_buf
            } else {
                0.0
            };
            self.allpass_indices[i] = (idx + 1) % buf.len();
            ap_out = out;
        }

        // Dry / Wet 결합 및 소프트 리미팅
        let mixed = dry * input + wet * ap_out;
        if mixed.is_finite() {
            mixed.clamp(-1.0, 1.0)
        } else {
            input
        }
    }
}

#[inline(always)]
fn soft_clip(x: f32) -> f32 {
    if !x.is_finite() {
        return 0.0;
    }
    let abs_x = x.abs();
    if abs_x <= 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.2 * ((abs_x - 0.8) / 0.2).tanh())
    }
}

/// rodio Source를 래핑하여 실시간 EQ, 리버브 및 비주얼라이저 버퍼를 처리하는 DSP 스트림
pub struct DspSource<I> {
    source: I,
    params: Arc<Mutex<DspParams>>,
    visualizer: Arc<Mutex<VisualizerState>>,
    sample_rate: u32,
    channels: u16,

    // 안정적인 캐시 파라미터 (락 경합 시 기본값 리셋 버그 원천 차단)
    cached_params: DspParams,

    // DSP 필터 인스턴스 (채널별)
    pitch_shifters: Vec<DualDelayPitchShifter>,
    bass_filters: Vec<BiquadFilter>,
    treble_filters: Vec<BiquadFilter>,
    reverbs: Vec<SimpleReverb>,

    last_bass_gain: f32,
    last_treble_gain: f32,
    current_channel: usize,
    check_counter: usize,
    current_volume: f32,
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

        let cached_params = params.lock().map(|p| p.clone()).unwrap_or_default();
        let current_volume = cached_params.volume;

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
            cached_params,
            pitch_shifters,
            bass_filters,
            treble_filters,
            reverbs,
            last_bass_gain: -999.0,
            last_treble_gain: -999.0,
            current_channel: 0,
            check_counter: 0,
            current_volume,
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

        // 64샘플 블록 단위 체크 (초당 ~1,378회): 락 경합 방지 및 CPU 오버헤드 98% 격감
        self.check_counter += 1;
        if self.check_counter % 64 == 0 {
            if let Ok(p) = self.params.try_lock() {
                self.cached_params = p.clone();
            }
        }

        let speed = self.cached_params.speed;
        let pitch_semi = self.cached_params.pitch_semitones;
        let vinyl = self.cached_params.vinyl_link;
        let bass_gain = self.cached_params.bass_gain_db;
        let treble_gain = self.cached_params.treble_gain_db;
        let reverb_pct = self.cached_params.reverb_pct;
        
        // 볼륨 스무딩 (지퍼 노이즈 방지)
        let target_vol = self.cached_params.volume;
        self.current_volume = self.current_volume * 0.98 + target_vol * 0.02;
        let vol = self.current_volume;

        if (bass_gain - self.last_bass_gain).abs() > 0.05 {
            self.last_bass_gain = bass_gain;
            for bf in self.bass_filters.iter_mut() {
                bf.set_low_shelf(self.sample_rate as f32, 160.0, bass_gain);
            }
        }

        if (treble_gain - self.last_treble_gain).abs() > 0.05 {
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

        // 안전 보간 및 글라이드 피치 시프터 적용
        let mut sample = self.pitch_shifters[ch].process(raw_sample, target_pitch);

        // 2. 베이스 부스트 필터 적용
        sample = self.bass_filters[ch].process(sample);

        // 3. 고음역 에어 / 트레블 부스트 필터 적용
        sample = self.treble_filters[ch].process(sample);

        // 4. 리버브 필터 적용
        sample = self.reverbs[ch].process(sample, reverb_pct);

        // 5. 볼륨 조절 및 아날로그형 소프트 클리핑 적용 (디지털 클리핑 찌직거림 차단)
        let final_sample = if sample.is_finite() {
            soft_clip(sample * vol).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        // 6. 비주얼라이저 버퍼에 첫 번째 채널 샘플 전송
        if ch == 0 {
            if let Ok(mut viz) = self.visualizer.try_lock() {
                viz.push_sample(final_sample);
            }
        }

        Some(final_sample)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soft_clip_characteristics() {
        assert_eq!(soft_clip(0.0), 0.0);
        assert_eq!(soft_clip(0.5), 0.5);
        assert_eq!(soft_clip(-0.5), -0.5);
        assert_eq!(soft_clip(0.8), 0.8);
        assert_eq!(soft_clip(-0.8), -0.8);

        // 0.8 초과 시 1.0 이하로 부드럽게 새츄레이션
        let clipped_pos = soft_clip(1.5);
        assert!(clipped_pos > 0.8 && clipped_pos <= 1.0);

        let clipped_neg = soft_clip(-1.5);
        assert!(clipped_neg < -0.8 && clipped_neg >= -1.0);

        // 비정상 수치(NaN, Inf) 방어
        assert_eq!(soft_clip(f32::NAN), 0.0);
        assert_eq!(soft_clip(f32::INFINITY), 0.0);
    }

    #[test]
    fn test_biquad_denormal_flush() {
        let mut filter = BiquadFilter::default();
        filter.set_low_shelf(44100.0, 160.0, 6.0);

        // 극소수(Denormal/Subnormal, < 1e-15) 입력 시 필터 상태가 0.0으로 플러시되는지 확인
        let _ = filter.process(1e-18);
        let _ = filter.process(1e-20);
        assert_eq!(filter.s1, 0.0);
        assert_eq!(filter.s2, 0.0);
    }

    #[test]
    fn test_pitch_shifter_glide_and_stability() {
        let mut shifter = DualDelayPitchShifter::new(44100.0);

        // 급격한 반음 변화에도 패닉 없이 부드럽게 샘플 출력
        for i in 0..500 {
            let semitones = if i < 250 { 12.0 } else { -12.0 };
            let out = shifter.process(0.5, semitones);
            assert!(out.is_finite());
            assert!(out.abs() <= 1.5);
        }
    }
}
