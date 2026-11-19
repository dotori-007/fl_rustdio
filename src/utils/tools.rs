use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
pub struct ToolPaths {
    pub ffmpeg: Option<PathBuf>,
    pub ytdlp: Option<PathBuf>,
}

impl ToolPaths {
    pub fn detect() -> Self {
        Self {
            ffmpeg: find_ffmpeg(),
            ytdlp: find_ytdlp(),
        }
    }
}

#[cfg(target_os = "windows")]
pub const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(target_os = "windows")]
pub const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x00004000;

/// Windows 환경에서 Command 실행 시 콘솔/cmd 창 팝업 방지 및 백그라운드 우선순위 설정
/// (FFmpeg CPU 점유율로 인한 실시간 오디오 재생 스트림 끊김 원천 방지)
pub fn configure_no_window(cmd: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS);
    }
}

pub fn find_ffmpeg() -> Option<PathBuf> {
    let candidates = [
        r"C:\2026591001\source\github_pr\summer_w3_java_team_project\tools\ffmpeg\ffmpeg.exe",
        r"C:\tools\ffmpeg\bin\ffmpeg.exe",
        r"C:\ffmpeg\bin\ffmpeg.exe",
        r".\tools\ffmpeg\ffmpeg.exe",
        r".\tools\ffmpeg.exe",
    ];

    for path_str in candidates {
        let p = Path::new(path_str);
        if p.exists() && p.is_file() {
            return Some(p.to_path_buf());
        }
    }

    // Try `where.exe ffmpeg`
    let mut where_cmd = Command::new("where.exe");
    where_cmd.arg("ffmpeg");
    configure_no_window(&mut where_cmd);
    if let Ok(output) = where_cmd.output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let trimmed = line.trim();
                let p = Path::new(trimmed);
                if p.exists() {
                    return Some(p.to_path_buf());
                }
            }
        }
    }

    None
}

pub fn find_ytdlp() -> Option<PathBuf> {
    let candidates = [
        r"C:\2026591001\Python_3.10.11_64\Scripts\yt-dlp.exe",
        r"C:\tools\yt-dlp\yt-dlp.exe",
        r".\tools\yt-dlp.exe",
    ];

    for path_str in candidates {
        let p = Path::new(path_str);
        if p.exists() && p.is_file() {
            return Some(p.to_path_buf());
        }
    }

    // Try `where.exe yt-dlp`
    let mut where_cmd = Command::new("where.exe");
    where_cmd.arg("yt-dlp");
    configure_no_window(&mut where_cmd);
    if let Ok(output) = where_cmd.output() {
        if output.status.success() {
            let out_str = String::from_utf8_lossy(&output.stdout);
            for line in out_str.lines() {
                let trimmed = line.trim();
                let p = Path::new(trimmed);
                if p.exists() {
                    return Some(p.to_path_buf());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_detection() {
        let tools = ToolPaths::detect();
        println!("Detected FFmpeg: {:?}", tools.ffmpeg);
        println!("Detected yt-dlp: {:?}", tools.ytdlp);
        assert!(tools.ffmpeg.is_some(), "FFmpeg should be detected");
    }
}
