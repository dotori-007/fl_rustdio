use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;
use super::engine::{probe_audio_info, AudioInfo};
use crate::utils::paths::{prune_directory_queue, MAX_DIRECTORY_QUEUE_FILES};
use crate::utils::tools::configure_no_window;

pub fn extract_audio_from_url(
    url: &str,
    output_dir: &Path,
    ytdlp_path: &Path,
    ffmpeg_path: &Path,
) -> Result<AudioInfo, String> {
    // 1. System32 방지 및 rust_mp3_editor/uploads 절대 경로 보장
    let abs_output_dir = {
        let is_sys32 = output_dir.to_string_lossy().to_lowercase().contains("system32");
        if output_dir.is_absolute() && !is_sys32 {
            output_dir.to_path_buf()
        } else {
            crate::utils::paths::get_uploads_dir()
        }
    };

    if !abs_output_dir.exists() {
        std::fs::create_dir_all(&abs_output_dir)
            .map_err(|e| format!("출력 폴더({:?}) 생성 실패: {e}", abs_output_dir))?;
    }

    let clean_id = &Uuid::new_v4().to_string()[..8];
    let output_template = abs_output_dir.join(format!("extracted_{clean_id}_%(title).80s.%(ext)s"));
    let output_template_str = output_template.to_string_lossy().to_string();

    let ffmpeg_dir = ffmpeg_path.parent().unwrap_or(ffmpeg_path);

    // 2. yt-dlp 명령어 실행 (고속 멀티스레드 다운로드 & CMD 창 팝업 방지)
    let mut cmd = Command::new(ytdlp_path);
    configure_no_window(&mut cmd);

    cmd.current_dir(&abs_output_dir)
        .arg("-f")
        .arg("ba/b") // 최적 오디오 스트림만 다운로드 (대용량 비디오 다운로드 원천 방지)
        .arg("-x")
        .arg("--audio-format")
        .arg("mp3")
        .arg("--audio-quality")
        .arg("192k")
        .arg("--concurrent-fragments")
        .arg("4") // 프래그먼트 4개 동시 병렬 다운로드 (속도 대폭 향상)
        .arg("--buffer-size")
        .arg("16K")
        .arg("--no-mtime")
        .arg("--windows-filenames")
        .arg("--no-playlist")
        .arg("--ffmpeg-location")
        .arg(ffmpeg_dir)
        .arg("-o")
        .arg(&output_template_str)
        .arg(url);

    let output = cmd.output().map_err(|e| format!("yt-dlp 실행 실패: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!("오디오 추출 실패:\n{}\n{}", stderr, stdout));
    }

    let prefix = format!("extracted_{clean_id}_");
    let mut target_file: Option<PathBuf> = None;

    if let Ok(entries) = std::fs::read_dir(&abs_output_dir) {
        for entry in entries.flatten() {
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.starts_with(&prefix) && fname.ends_with(".mp3") {
                target_file = Some(entry.path());
                break;
            }
        }
    }

    let file_path = target_file.ok_or_else(|| "추출된 MP3 파일을 찾을 수 없습니다.".to_string())?;

    // 업로드(uploads) 폴더 10개 FIFO 큐 용량 관리 (방금 추출한 file_path 보존)
    prune_directory_queue(&abs_output_dir, MAX_DIRECTORY_QUEUE_FILES, Some(&file_path));

    probe_audio_info(&file_path, ffmpeg_path)
}
