use std::path::PathBuf;
use std::thread;
use crossbeam_channel::{Receiver, Sender};
use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke};
use crate::audio::engine::{merge_audio_files, process_audio, AudioFormat, AudioInfo, AudioProcessParams};
use crate::audio::extractor::extract_audio_from_url;
use crate::ui::tab_editor::{render_editor_tab, EditorTabState};
use crate::ui::tab_extract::ExtractTabState;
use crate::ui::tab_merge::{render_merge_tab, MergeTabState};
use crate::ui::waveform::render_bottom_waveform;
use crate::utils::paths::get_uploads_dir;
use crate::ui::theme::{
    COLOR_ACCENT_AMBER, COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN, COLOR_ACCENT_PURPLE, COLOR_ACCENT_RED,
    COLOR_BG_BASE, COLOR_BG_CARD_SUB, COLOR_BG_PANEL, COLOR_BORDER, COLOR_TEXT_MUTED,
};
use crate::utils::tools::ToolPaths;

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum AppTab {
    Editor,
    Merge,
}

pub enum WorkerResult {
    ProcessDone(Result<AudioInfo, String>),
    ExtractDone(Result<AudioInfo, String>),
    MergeDone(Result<AudioInfo, String>),
}

pub struct Mp3EditorApp {
    pub active_tab: AppTab,
    pub tools: ToolPaths,
    pub tab_editor: EditorTabState,
    pub tab_extract: ExtractTabState,
    pub tab_merge: MergeTabState,

    tx_process: Sender<(PathBuf, PathBuf, AudioProcessParams)>,
    tx_extract: Sender<String>,
    tx_merge: Sender<(Vec<PathBuf>, PathBuf, AudioFormat)>,
    rx_result: Receiver<WorkerResult>,
}

impl Mp3EditorApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let tools = ToolPaths::detect();

        let (tx_process, rx_process) = crossbeam_channel::unbounded::<(PathBuf, PathBuf, AudioProcessParams)>();
        let (tx_extract, rx_extract) = crossbeam_channel::unbounded::<String>();
        let (tx_merge, rx_merge) = crossbeam_channel::unbounded::<(Vec<PathBuf>, PathBuf, AudioFormat)>();
        let (tx_result, rx_result) = crossbeam_channel::unbounded::<WorkerResult>();

        // 1. Process Worker Thread
        {
            let ffmpeg_opt = tools.ffmpeg.clone();
            let tx_res = tx_result.clone();
            thread::spawn(move || {
                while let Ok((in_p, out_p, params)) = rx_process.recv() {
                    let res = if let Some(ref ffmpeg) = ffmpeg_opt {
                        process_audio(&in_p, &out_p, &params, ffmpeg)
                    } else {
                        Err("FFmpeg를 찾을 수 없습니다.".to_string())
                    };
                    let _ = tx_res.send(WorkerResult::ProcessDone(res));
                }
            });
        }

        // 2. Extract Worker Thread
        {
            let ffmpeg_opt = tools.ffmpeg.clone();
            let ytdlp_opt = tools.ytdlp.clone();
            let tx_res = tx_result.clone();
            thread::spawn(move || {
                let uploads_dir = get_uploads_dir();

                while let Ok(url) = rx_extract.recv() {
                    let res = match (&ytdlp_opt, &ffmpeg_opt) {
                        (Some(ytdlp), Some(ffmpeg)) => {
                            extract_audio_from_url(&url, &uploads_dir, ytdlp, ffmpeg)
                        }
                        _ => Err("yt-dlp 또는 FFmpeg를 찾을 수 없습니다.".to_string()),
                    };
                    let _ = tx_res.send(WorkerResult::ExtractDone(res));
                }
            });
        }

        // 3. Merge Worker Thread
        {
            let ffmpeg_opt = tools.ffmpeg.clone();
            let tx_res = tx_result.clone();
            thread::spawn(move || {
                while let Ok((inputs, out_p, fmt)) = rx_merge.recv() {
                    let res = if let Some(ref ffmpeg) = ffmpeg_opt {
                        merge_audio_files(&inputs, &out_p, fmt, ffmpeg)
                    } else {
                        Err("FFmpeg를 찾을 수 없습니다.".to_string())
                    };
                    let _ = tx_res.send(WorkerResult::MergeDone(res));
                }
            });
        }

        Self {
            active_tab: AppTab::Editor,
            tools,
            tab_editor: EditorTabState::default(),
            tab_extract: ExtractTabState::default(),
            tab_merge: MergeTabState::default(),
            tx_process,
            tx_extract,
            tx_merge,
            rx_result,
        }
    }

    fn check_worker_results(&mut self) {
        while let Ok(result) = self.rx_result.try_recv() {
            match result {
                WorkerResult::ProcessDone(res) => {
                    self.tab_editor.is_processing = false;
                    match res {
                        Ok(info) => {
                            let _ = self.tab_editor.result_player.load_file(&info.file_path, info.duration);
                            self.tab_editor.result_info = Some(info);
                            self.tab_editor.process_error = None;
                        }
                        Err(err) => {
                            self.tab_editor.process_error = Some(err);
                        }
                    }
                }
                WorkerResult::ExtractDone(res) => {
                    self.tab_extract.is_extracting = false;
                    self.tab_extract.status_message = None;
                    match res {
                        Ok(info) => {
                            let _ = self.tab_extract.player.load_file(&info.file_path, info.duration);
                            // 추출 즉시 좌측 편집기로 자동 로드
                            self.tab_editor.load_file(info.file_path.clone(), &self.tools);
                            self.tab_extract.extracted_info = Some(info);
                            self.tab_extract.error_message = None;
                        }
                        Err(err) => {
                            self.tab_extract.error_message = Some(err);
                        }
                    }
                }
                WorkerResult::MergeDone(res) => {
                    self.tab_merge.is_merging = false;
                    match res {
                        Ok(info) => {
                            let _ = self.tab_merge.result_player.load_file(&info.file_path, info.duration);
                            self.tab_merge.result_info = Some(info);
                            self.tab_merge.error_message = None;
                        }
                        Err(err) => {
                            self.tab_merge.error_message = Some(err);
                        }
                    }
                }
            }
        }
    }
}

impl eframe::App for Mp3EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_worker_results();

        // [스페이스바(Space) 단축키]: 오디오 재생 / 일시정지 토글
        // 텍스트 인풋(URL 필드 등)에 포커스가 있을 때는 스페이스 공백 입력을 위해 가로채지 않음
        if !ctx.wants_keyboard_input() && ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            let active_player = if self.tab_editor.result_player.is_playing {
                Some(&mut self.tab_editor.result_player)
            } else if self.tab_merge.result_player.is_playing {
                Some(&mut self.tab_merge.result_player)
            } else if self.tab_extract.player.is_playing {
                Some(&mut self.tab_extract.player)
            } else if self.tab_editor.source_player.is_playing {
                Some(&mut self.tab_editor.source_player)
            } else {
                // 모두 정지 상태일 때: 현재 탭 기준 파일이 로드된 플레이어 실행
                match self.active_tab {
                    AppTab::Editor => {
                        if self.tab_editor.source_player.duration > std::time::Duration::ZERO {
                            Some(&mut self.tab_editor.source_player)
                        } else if self.tab_editor.result_player.duration > std::time::Duration::ZERO {
                            Some(&mut self.tab_editor.result_player)
                        } else {
                            None
                        }
                    }
                    AppTab::Merge => {
                        if self.tab_merge.result_player.duration > std::time::Duration::ZERO {
                            Some(&mut self.tab_merge.result_player)
                        } else {
                            None
                        }
                    }
                }
            };

            if let Some(player) = active_player {
                let _ = player.toggle_play();
            }
        }

        let is_any_playing = self.tab_editor.source_player.is_playing
            || self.tab_editor.result_player.is_playing
            || self.tab_extract.player.is_playing
            || self.tab_merge.result_player.is_playing;

        if is_any_playing {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS 비주얼라이저 애니메이션
        } else if self.tab_editor.is_processing
            || self.tab_extract.is_extracting
            || self.tab_merge.is_merging
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(40)); // 대기 모드 25 FPS 은은한 물결 애니메이션
        }

        // 상단 슬림 프리미엄 헤더
        egui::TopBottomPanel::top("top_header")
            .frame(
                Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .inner_margin(Margin::symmetric(16.0, 10.0))
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // 앱 타이틀 & 아이콘 (Windows 표준 호환 폰트 글리프 적용)
                    ui.label(RichText::new("🎵 PureRust Audio Studio").size(16.5).strong().color(COLOR_ACCENT_CYAN));
                    ui.add_space(16.0);

                    // 세그먼트 스타일 네비게이션 탭
                    let tab_btn = |ui: &mut egui::Ui, icon: &str, name: &str, tab: AppTab, cur: AppTab| -> bool {
                        let is_active = tab == cur;
                        let text = if is_active {
                            RichText::new(format!("{icon} {name}")).strong().color(Color32::WHITE).size(13.0)
                        } else {
                            RichText::new(format!("{icon} {name}")).color(COLOR_TEXT_MUTED).size(13.0)
                        };
                        let fill = if is_active {
                            COLOR_ACCENT_PURPLE
                        } else {
                            COLOR_BG_CARD_SUB
                        };
                        let stroke = if is_active {
                            Stroke::new(1.0_f32, Color32::from_rgb(167, 139, 250))
                        } else {
                            Stroke::new(1.0_f32, COLOR_BORDER)
                        };

                        let btn = egui::Button::new(text)
                            .fill(fill)
                            .stroke(stroke)
                            .rounding(Rounding::same(6.0));
                        ui.add(btn).clicked()
                    };

                    if tab_btn(ui, "🎧", "트랙 편집", AppTab::Editor, self.active_tab) {
                        self.active_tab = AppTab::Editor;
                    }
                    if tab_btn(ui, "🔗", "트랙 병합", AppTab::Merge, self.active_tab) {
                        self.active_tab = AppTab::Merge;
                    }

                    // 우측 엔진 상태 인디케이터 (FFmpeg / yt-dlp)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // yt-dlp 뱃지
                        let (ytdlp_text, ytdlp_color) = if self.tools.ytdlp.is_some() {
                            ("● yt-dlp 가용", COLOR_ACCENT_GREEN)
                        } else {
                            ("○ yt-dlp 없음", COLOR_ACCENT_AMBER)
                        };
                        ui.label(RichText::new(ytdlp_text).color(ytdlp_color).size(11.5).strong());

                        ui.add_space(8.0);

                        // FFmpeg 뱃지
                        let (ffmpeg_text, ffmpeg_color) = if self.tools.ffmpeg.is_some() {
                            ("● FFmpeg 가용", COLOR_ACCENT_GREEN)
                        } else {
                            ("○ FFmpeg 누락", COLOR_ACCENT_RED)
                        };
                        ui.label(RichText::new(ffmpeg_text).color(ffmpeg_color).size(11.5).strong());
                    });
                });
            });

        // 하단 실시간 오디오 전체 음 파형 타임라인 도크
        egui::TopBottomPanel::bottom("bottom_waveform")
            .frame(Frame::none())
            .show(ctx, |ui| {
                let (active_player, track_name) = if self.tab_editor.result_player.is_playing {
                    (
                        &mut self.tab_editor.result_player,
                        self.tab_editor.result_info.as_ref().map(|i| i.filename.as_str()),
                    )
                } else if self.tab_merge.result_player.is_playing {
                    (
                        &mut self.tab_merge.result_player,
                        self.tab_merge.result_info.as_ref().map(|i| i.filename.as_str()),
                    )
                } else if self.tab_extract.player.is_playing {
                    (
                        &mut self.tab_extract.player,
                        self.tab_extract.extracted_info.as_ref().map(|i| i.filename.as_str()),
                    )
                } else {
                    (
                        &mut self.tab_editor.source_player,
                        self.tab_editor.source_info.as_ref().map(|i| i.filename.as_str()),
                    )
                };

                render_bottom_waveform(ui, active_player, track_name);
            });

        // 메인 패널
        egui::CentralPanel::default()
            .frame(Frame::none().fill(COLOR_BG_BASE).inner_margin(Margin::same(14.0)))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        AppTab::Editor => {
                            render_editor_tab(
                                ui,
                                &mut self.tab_editor,
                                &mut self.tab_extract,
                                &self.tools,
                                &self.tx_process,
                                &self.tx_extract,
                            );
                        }
                        AppTab::Merge => {
                            render_merge_tab(ui, &mut self.tab_merge, &self.tools, &self.tx_merge);
                        }
                    }
                });
            });
    }
}
