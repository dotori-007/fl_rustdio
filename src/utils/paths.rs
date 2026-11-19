use std::path::PathBuf;

/// 프로젝트 루트(rust_mp3_editor) 디렉토리 감지 (System32 방지)
pub fn get_project_root_dir() -> PathBuf {
    // 1. 현재 실행 중인 exe 파일 위치 기반 탐색
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            // target/release 또는 target/debug 내부에서 실행된 경우: 상위 2단계가 프로젝트 루트
            if exe_dir.ends_with("release") || exe_dir.ends_with("debug") {
                if let Some(target_dir) = exe_dir.parent() {
                    if let Some(project_root) = target_dir.parent() {
                        if project_root.join("Cargo.toml").exists() || project_root.join("src").exists() {
                            return project_root.to_path_buf();
                        }
                    }
                }
            }

            // exe와 같은 디렉토리에 Cargo.toml이 있는 경우
            if exe_dir.join("Cargo.toml").exists() {
                return exe_dir.to_path_buf();
            }

            // exe_dir 자체가 유효한 디렉토리인 경우 (System32 / Windows 제외)
            let s = exe_dir.to_string_lossy().to_lowercase();
            if !s.contains("system32") && !s.contains("windows") {
                return exe_dir.to_path_buf();
            }
        }
    }

    // 2. 지정된 프로젝트 디렉토리 확인 (fl_rustdio / rust_mp3_editor)
    let candidates = [
        PathBuf::from(r"C:\2026591001\source\github_pr\fl_rustdio"),
        PathBuf::from(r"C:\2026591001\source\github_pr\rust_mp3_editor"),
    ];
    for cand in &candidates {
        if cand.exists() {
            return cand.clone();
        }
    }

    // 3. 작업 디렉토리 (System32가 아닐 때만 허용)
    if let Ok(cur) = std::env::current_dir() {
        let s = cur.to_string_lossy().to_lowercase();
        if !s.contains("system32") && !s.contains("windows") {
            return cur;
        }
    }

    candidates[0].clone()
}

pub const MAX_DIRECTORY_QUEUE_FILES: usize = 10;

/// 항상 안전한 rust_mp3_editor/uploads 절대 경로 반환 및 생성
pub fn get_uploads_dir() -> PathBuf {
    let dir = get_project_root_dir().join("uploads");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 항상 안전한 rust_mp3_editor/outputs 절대 경로 반환 및 생성
pub fn get_outputs_dir() -> PathBuf {
    let dir = get_project_root_dir().join("outputs");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// 디렉토리 내 파일 개수를 FIFO 큐(Queue) 형태로 관리하여,
/// 파일 수가 `max_items`(기본 10개)를 초과할 경우 가장 오래된 파일부터 삭제합니다.
/// `preserve_path`가 주어지면 해당 파일(방금 생성된 파일 등)은 절대 삭제되지 않습니다.
pub fn prune_directory_queue(dir: &std::path::Path, max_items: usize, preserve_path: Option<&std::path::Path>) {
    if !dir.exists() || !dir.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    let mut files: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let mut has_preserved = false;

    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                let path = entry.path();
                if let Some(preserve) = preserve_path {
                    if path == preserve {
                        has_preserved = true;
                        continue;
                    }
                }
                let time = metadata
                    .modified()
                    .or_else(|_| metadata.created())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                files.push((path, time));
            }
        }
    }

    let total_count = files.len() + if has_preserved { 1 } else { 0 };
    if total_count > max_items {
        let remove_count = total_count - max_items;
        // 오래된 순으로 정렬 (가장 오래된 파일이 0번 인덱스)
        files.sort_by(|a, b| a.1.cmp(&b.1));

        for (path, _) in files.iter().take(remove_count) {
            let _ = std::fs::remove_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project_root_not_system32() {
        let root = get_project_root_dir();
        let s = root.to_string_lossy().to_lowercase();
        assert!(!s.contains("system32"), "Project root must not be System32");

        let uploads = get_uploads_dir();
        assert!(uploads.exists(), "Uploads directory must exist");
        assert!(uploads.to_string_lossy().ends_with("uploads"));

        let outputs = get_outputs_dir();
        assert!(outputs.exists(), "Outputs directory must exist");
        assert!(outputs.to_string_lossy().ends_with("outputs"));
    }

    #[test]
    fn test_prune_directory_queue() {
        let temp_dir = std::env::temp_dir().join(format!("test_queue_{}", uuid::Uuid::new_v4()));
        let _ = std::fs::create_dir_all(&temp_dir);

        // 13개의 파일 생성
        let mut created_files = Vec::new();
        for i in 0..13 {
            let p = temp_dir.join(format!("test_file_{:02}.mp3", i));
            let _ = std::fs::write(&p, format!("dummy content {}", i));
            created_files.push(p);
            std::thread::sleep(std::time::Duration::from_millis(15)); // 시간 차이 보장
        }

        let preserve_file = created_files.last().cloned();

        // 최대 10개 큐로 정리
        prune_directory_queue(&temp_dir, 10, preserve_file.as_deref());

        let remaining: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .flatten()
            .collect();

        assert_eq!(remaining.len(), 10, "남은 파일 수는 정확히 10개여야 함");

        // 가장 최신 파일 및 보존 대상 파일은 살아있어야 함
        if let Some(preserved) = preserve_file {
            assert!(preserved.exists(), "보존 대상 파일은 삭제되지 않아야 함");
        }

        // 임시 폴더 정리
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
