// PureRust MP3 Editor Studio - Pure Rust Native Application
// Windows GUI Subsystem (콘솔창 숨기기 옵션 지원)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod ui;
mod utils;

use app::Mp3EditorApp;
use eframe::egui;
use ui::theme::{apply_dark_studio_theme, setup_custom_fonts};
use utils::paths::{get_outputs_dir, get_uploads_dir, prune_directory_queue, MAX_DIRECTORY_QUEUE_FILES};

fn main() -> eframe::Result<()> {
    // 필수 작업 폴더 준비 및 10개 초과 파일 FIFO 큐 정리 (용량 관리)
    let uploads_dir = get_uploads_dir();
    let outputs_dir = get_outputs_dir();
    prune_directory_queue(&uploads_dir, MAX_DIRECTORY_QUEUE_FILES, None);
    prune_directory_queue(&outputs_dir, MAX_DIRECTORY_QUEUE_FILES, None);

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("PureRust MP3 Editor Studio")
            .with_inner_size([960.0, 710.0])
            .with_min_inner_size([800.0, 560.0])
            .with_drag_and_drop(true),
        ..Default::default()
    };

    eframe::run_native(
        "PureRust MP3 Editor Studio",
        native_options,
        Box::new(|cc| {
            // 한글 폰트 및 다크 오디오 스튜디오 테마 초기화
            setup_custom_fonts(&cc.egui_ctx);
            apply_dark_studio_theme(&cc.egui_ctx);

            Ok(Box::new(Mp3EditorApp::new(cc)))
        }),
    )
}
