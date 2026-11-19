use std::path::{Path, PathBuf};
use std::process::Command;
use crate::utils::paths::{prune_directory_queue, MAX_DIRECTORY_QUEUE_FILES};
use crate::utils::tools::configure_no_window;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AudioFormat {
    #[default]
    Mp3,
    Wav,
    Flac,
}

impl AudioFormat {
    pub const ALL: [AudioFormat; 3] = [AudioFormat::Mp3, AudioFormat::Wav, AudioFormat::Flac];

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Wav => "wav",
            Self::Flac => "flac",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Mp3 => "MP3 (고음질)",
            Self::Wav => "WAV (무손실 PCM)",
            Self::Flac => "FLAC (무손실 압축)",
        }
    }

    pub fn is_lossless(&self) -> bool {
        matches!(self, Self::Wav | Self::Flac)
    }

    pub fn codec_args(&self) -> Vec<&'static str> {
        match self {
            Self::Mp3 => vec!["-c:a", "libmp3lame", "-q:a", "2"],
            Self::Wav => vec!["-c:a", "pcm_s16le"],
            Self::Flac => vec!["-c:a", "flac"],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GenrePreset {
    #[default]
    Custom,
    ClassicNightcore,
    ModernNightcore,
    LolicoreHyper,
    LolicoreExtreme,
    SlowedReverb,
}

impl GenrePreset {
    pub const ALL: [GenrePreset; 6] = [
        GenrePreset::Custom,
        GenrePreset::ClassicNightcore,
        GenrePreset::ModernNightcore,
        GenrePreset::LolicoreHyper,
        GenrePreset::LolicoreExtreme,
        GenrePreset::SlowedReverb,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::Custom => "사용자 정의 (Custom)",
            Self::ClassicNightcore => "⚡ Classic Nightcore (바이닐 가속)",
            Self::ModernNightcore => "✨ Modern Nightcore (보컬 튜닝)",
            Self::LolicoreHyper => "🔥 Lolicore Hyper (200+ BPM)",
            Self::LolicoreExtreme => "⚡ Lolicore Extreme (초고속)",
            Self::SlowedReverb => "🌙 Slowed & Reverb (드림코어)",
        }
    }
}

#[derive(Clone, Debug)]
pub struct AudioProcessParams {
    pub speed: f32,
    pub pitch_semitones: f32,
    pub vinyl_link: bool,
    pub bass_gain_db: f32,
    pub treble_gain_db: f32,
    pub reverb_pct: f32,
    pub format: AudioFormat,
}

impl Default for AudioProcessParams {
    fn default() -> Self {
        Self {
            speed: 1.0,
            pitch_semitones: 0.0,
            vinyl_link: false,
            bass_gain_db: 0.0,
            treble_gain_db: 0.0,
            reverb_pct: 0.0,
            format: AudioFormat::Mp3,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AudioInfo {
    pub file_path: PathBuf,
    pub filename: String,
    pub duration: f64,
    pub duration_formatted: String,
    pub bitrate: String,
    pub size_bytes: u64,
    #[allow(dead_code)]
    pub title: Option<String>,
}

pub fn format_duration(seconds: f64) -> String {
    let total_secs = seconds.round() as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{mins:02}:{secs:02}")
}

pub fn build_speed_filters(speed: f32) -> Vec<String> {
    let mut filters = Vec::new();
    let mut current_speed = if speed <= 0.0 { 1.0 } else { speed };

    while current_speed > 2.0 {
        filters.push("atempo=2.0".to_string());
        current_speed /= 2.0;
    }
    while current_speed < 0.5 {
        filters.push("atempo=0.5".to_string());
        current_speed /= 0.5;
    }

    if (current_speed - 1.0).abs() > 0.001 {
        filters.push(format!("atempo={:.4}", current_speed));
    }
    filters
}

pub fn probe_audio_info(file_path: &Path, ffmpeg_path: &Path) -> Result<AudioInfo, String> {
    if !file_path.exists() {
        return Err(format!("파일을 찾을 수 없습니다: {}", file_path.display()));
    }

    let mut cmd = Command::new(ffmpeg_path);
    configure_no_window(&mut cmd);
    let output = cmd
        .arg("-i")
        .arg(file_path)
        .output()
        .map_err(|e| format!("FFmpeg 실행 실패: {e}"))?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut duration = 0.0f64;
    let mut bitrate = "N/A".to_string();

    for line in stderr.lines() {
        if let Some(pos) = line.find("Duration: ") {
            let rest = &line[pos + 10..];
            if let Some(comma_pos) = rest.find(',') {
                let dur_str = rest[..comma_pos].trim();
                let parts: Vec<&str> = dur_str.split(':').collect();
                if parts.len() == 3 {
                    if let (Ok(h), Ok(m), Ok(s)) = (
                        parts[0].parse::<f64>(),
                        parts[1].parse::<f64>(),
                        parts[2].parse::<f64>(),
                    ) {
                        duration = h * 3600.0 + m * 60.0 + s;
                    }
                }
            }

            if let Some(br_pos) = line.find("bitrate: ") {
                let br_rest = &line[br_pos + 9..];
                bitrate = br_rest.split_whitespace().next().unwrap_or("N/A").to_string();
                if !bitrate.ends_with("b/s") {
                    bitrate.push_str(" kb/s");
                }
            }
        }
    }

    let size_bytes = std::fs::metadata(file_path)
        .map(|m| m.len())
        .unwrap_or(0);

    let filename = file_path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "audio".to_string());

    Ok(AudioInfo {
        file_path: file_path.to_path_buf(),
        filename,
        duration,
        duration_formatted: format_duration(duration),
        bitrate,
        size_bytes,
        title: None,
    })
}

pub fn process_audio(
    input_path: &Path,
    output_path: &Path,
    params: &AudioProcessParams,
    ffmpeg_path: &Path,
) -> Result<AudioInfo, String> {
    if !input_path.exists() {
        return Err(format!("입력 파일이 존재하지 않습니다: {}", input_path.display()));
    }

    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut audio_filters = Vec::new();

    // 1. 속도 & 피치 (바이닐 연동 모드 vs 독립 조절 모드)
    if params.vinyl_link {
        // 바이닐/테이프 가속 리샘플링: 속도와 피치가 정비례하여 자연스럽게 함께 올라감
        let current_speed = if params.speed <= 0.0 { 1.0 } else { params.speed };
        let sample_rate = 44100;
        let target_rate = (sample_rate as f32 * current_speed).round() as u32;
        audio_filters.push(format!("asetrate={target_rate}"));
        audio_filters.push(format!("aresample={sample_rate}"));
    } else {
        // 독립 피치 시프트
        if params.pitch_semitones.abs() > 0.01 {
            let pitch_ratio = 2.0f64.powf((params.pitch_semitones / 12.0) as f64);
            let sample_rate = 44100;
            let target_rate = (sample_rate as f64 * pitch_ratio).round() as u32;
            audio_filters.push(format!("asetrate={target_rate}"));
            audio_filters.push(format!("aresample={sample_rate}"));
            let compensatory_speed = (1.0 / pitch_ratio) as f32;
            audio_filters.extend(build_speed_filters(compensatory_speed));
        }

        // 독립 템포 조절
        if (params.speed - 1.0).abs() > 0.01 {
            audio_filters.extend(build_speed_filters(params.speed));
        }
    }

    // 2. 베이스 부스트 (100Hz 중심 저음 쉘빙)
    if params.bass_gain_db.abs() > 0.1 {
        audio_filters.push(format!("bass=g={:.1}:f=100:w=0.5", params.bass_gain_db));
    }

    // 3. 고음역 에어 / 트레블 부스트 (8kHz 중심 고음 쉘빙) - 나이트코어 보컬 선명화
    if params.treble_gain_db.abs() > 0.1 {
        audio_filters.push(format!("treble=g={:.1}:f=8000:w=0.5", params.treble_gain_db));
    }

    // 4. 공간감 리버브 (Dreamy Reverb / Ambience)
    if params.reverb_pct > 1.0 {
        let decay = (params.reverb_pct / 100.0 * 0.6).clamp(0.05, 0.55);
        audio_filters.push(format!("aecho=0.8:0.88:40:{:.2}", decay));
    }

    let mut cmd = Command::new(ffmpeg_path);
    configure_no_window(&mut cmd);
    cmd.arg("-y").arg("-i").arg(input_path);

    if !audio_filters.is_empty() {
        let filter_str = audio_filters.join(",");
        cmd.arg("-af").arg(filter_str);
    }

    // 출력 포맷 코덱 적용
    for arg in params.format.codec_args() {
        cmd.arg(arg);
    }
    cmd.arg(output_path);

    let output = cmd.output().map_err(|e| format!("FFmpeg 실행 실패: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("오디오 인코딩 오류:\n{}", stderr));
    }

    // 출력(outputs) 폴더 10개 FIFO 큐 용량 관리 (방금 생성된 output_path 보존)
    if let Some(parent) = output_path.parent() {
        prune_directory_queue(parent, MAX_DIRECTORY_QUEUE_FILES, Some(output_path));
    }

    probe_audio_info(output_path, ffmpeg_path)
}

pub fn merge_audio_files(
    input_paths: &[PathBuf],
    output_path: &Path,
    format: AudioFormat,
    ffmpeg_path: &Path,
) -> Result<AudioInfo, String> {
    if input_paths.is_empty() {
        return Err("병합할 파일이 지정되지 않았습니다.".to_string());
    }

    if input_paths.len() == 1 {
        let default_params = AudioProcessParams {
            format,
            ..Default::default()
        };
        return process_audio(&input_paths[0], output_path, &default_params, ffmpeg_path);
    }

    if let Some(parent) = output_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let mut cmd = Command::new(ffmpeg_path);
    configure_no_window(&mut cmd);
    cmd.arg("-y");

    for p in input_paths {
        if !p.exists() {
            return Err(format!("파일이 존재하지 않습니다: {}", p.display()));
        }
        cmd.arg("-i").arg(p);
    }

    let filter_inputs: String = (0..input_paths.len())
        .map(|i| format!("[{i}:a]"))
        .collect();
    let filter_complex = format!("{}concat=n={}:v=0:a=1[outa]", filter_inputs, input_paths.len());

    cmd.arg("-filter_complex")
        .arg(filter_complex)
        .arg("-map")
        .arg("[outa]");

    for arg in format.codec_args() {
        cmd.arg(arg);
    }
    cmd.arg(output_path);

    let output = cmd.output().map_err(|e| format!("FFmpeg 실행 실패: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("오디오 병합 오류:\n{}", stderr));
    }

    // 출력(outputs) 폴더 10개 FIFO 큐 용량 관리 (방금 병합 생성된 output_path 보존)
    if let Some(parent) = output_path.parent() {
        prune_directory_queue(parent, MAX_DIRECTORY_QUEUE_FILES, Some(output_path));
    }

    probe_audio_info(output_path, ffmpeg_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(0.0), "00:00");
        assert_eq!(format_duration(65.0), "01:05");
        assert_eq!(format_duration(3665.0), "61:05");
    }

    #[test]
    fn test_speed_filters() {
        let f_normal = build_speed_filters(1.0);
        assert!(f_normal.is_empty());

        let f_1_5 = build_speed_filters(1.5);
        assert_eq!(f_1_5, vec!["atempo=1.5000"]);

        let f_2_5 = build_speed_filters(2.5);
        assert_eq!(f_2_5, vec!["atempo=2.0", "atempo=1.2500"]);

        let f_0_25 = build_speed_filters(0.25);
        assert_eq!(f_0_25, vec!["atempo=0.5", "atempo=0.5000"]);
    }

    #[test]
    fn test_audio_processing_integration() {
        use crate::utils::tools::find_ffmpeg;
        let ffmpeg = find_ffmpeg().expect("FFmpeg required for test");
        let sample_a = std::path::Path::new(r"C:\2026591001\source\github_pr\simple-mp3-editor\outputs\sample_a.mp3");
        if sample_a.exists() {
            // 1. 나이트코어 세팅 (배속 1.25, 피치 +3, 베이스 +4, 트레블 +3, 리버브 15%)
            let params_nc = AudioProcessParams {
                speed: 1.25,
                pitch_semitones: 3.0,
                vinyl_link: false,
                bass_gain_db: 4.0,
                treble_gain_db: 3.0,
                reverb_pct: 15.0,
                format: AudioFormat::Mp3,
            };
            let out_test_nc = std::path::Path::new(r"outputs\test_nightcore.mp3");
            let res = process_audio(sample_a, out_test_nc, &params_nc, &ffmpeg);
            assert!(res.is_ok(), "Nightcore process failed: {:?}", res.err());
            assert!(out_test_nc.exists());
            let _ = std::fs::remove_file(out_test_nc);

            // 2. 바이닐 연동 모드 (속도-피치 일체형)
            let params_vinyl = AudioProcessParams {
                speed: 1.25,
                pitch_semitones: 0.0,
                vinyl_link: true,
                bass_gain_db: 2.0,
                treble_gain_db: 2.0,
                reverb_pct: 0.0,
                format: AudioFormat::Wav,
            };
            let out_test_vinyl = std::path::Path::new(r"outputs\test_vinyl.wav");
            let res_vinyl = process_audio(sample_a, out_test_vinyl, &params_vinyl, &ffmpeg);
            assert!(res_vinyl.is_ok(), "Vinyl link process failed: {:?}", res_vinyl.err());
            assert!(out_test_vinyl.exists());
            let _ = std::fs::remove_file(out_test_vinyl);
        }
    }
}
