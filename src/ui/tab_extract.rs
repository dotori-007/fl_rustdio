use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};
use rfd::FileDialog;
use crate::audio::engine::AudioInfo;
use crate::audio::player::AudioPlayer;
use crate::utils::tools::ToolPaths;
use super::theme::{
    accent_card_frame, modern_card_frame, COLOR_ACCENT_AMBER, COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN,
    COLOR_ACCENT_PURPLE, COLOR_ACCENT_RED, COLOR_BG_CARD_SUB, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use super::widgets::render_audio_player;

pub struct ExtractTabState {
    pub url_input: String,
    pub is_extracting: bool,
    pub status_message: Option<String>,
    pub error_message: Option<String>,
    pub extracted_info: Option<AudioInfo>,
    pub player: AudioPlayer,
}

impl Default for ExtractTabState {
    fn default() -> Self {
        Self {
            url_input: String::new(),
            is_extracting: false,
            status_message: None,
            error_message: None,
            extracted_info: None,
            player: AudioPlayer::new(),
        }
    }
}

#[allow(dead_code)]
pub fn render_extract_tab(
    ui: &mut Ui,
    state: &mut ExtractTabState,
    tools: &ToolPaths,
    tx_extract: &crossbeam_channel::Sender<String>,
    on_send_to_editor: &mut Option<AudioInfo>,
) {
    ui.vertical(|ui| {
        // 상단 URL 입력 카드
        modern_card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🌐 온라인 URL 음원 추출").size(15.0).strong().color(COLOR_ACCENT_PURPLE));
                ui.label(RichText::new("YouTube / SoundCloud 지원").size(11.5).color(COLOR_TEXT_MUTED));
            });

            ui.add_space(8.0);

            ui.horizontal(|ui| {
                let text_edit = egui::TextEdit::singleline(&mut state.url_input)
                    .hint_text("추출할 YouTube 또는 SoundCloud 음원 URL을 입력하세요...")
                    .desired_width(ui.available_width() - 120.0);
                ui.add(text_edit);

                let can_extract = !state.url_input.trim().is_empty() 
                    && !state.is_extracting 
                    && tools.ytdlp.is_some() 
                    && tools.ffmpeg.is_some();

                let extract_btn = egui::Button::new(RichText::new("⬇ 추출 시작").strong().size(12.5).color(Color32::WHITE))
                    .fill(if can_extract { COLOR_ACCENT_PURPLE } else { COLOR_BG_CARD_SUB })
                    .stroke(Stroke::new(1.0_f32, if can_extract { Color32::from_rgb(167, 139, 250) } else { COLOR_BORDER }))
                    .rounding(Rounding::same(6.0));

                if ui.add_enabled(can_extract, extract_btn).clicked() {
                    state.is_extracting = true;
                    state.error_message = None;
                    state.status_message = Some("yt-dlp로 음원을 스트리밍 다운로드 및 인코딩하는 중입니다...".to_string());
                    let _ = tx_extract.send(state.url_input.trim().to_string());
                }
            });

            if tools.ytdlp.is_none() {
                ui.add_space(6.0);
                Frame::none()
                    .fill(Color32::from_rgb(38, 30, 18))
                    .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_AMBER))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::same(8.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new("⚠️ yt-dlp.exe가 탐지되지 않아 온라인 추출 기능을 사용할 수 없습니다.").size(11.5).color(COLOR_ACCENT_AMBER).strong());
                    });
            }
        });

        ui.add_space(10.0);

        // 추출 진행 중 스피너
        if state.is_extracting {
            Frame::none()
                .fill(COLOR_BG_CARD_SUB)
                .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_CYAN))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        let msg = state.status_message.as_deref().unwrap_or("음원 다운로드 중...");
                        ui.label(RichText::new(msg).color(COLOR_ACCENT_CYAN).strong());
                    });
                });
        }

        // 에러 메시지
        if let Some(ref err) = state.error_message {
            Frame::none()
                .fill(Color32::from_rgb(38, 18, 22))
                .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_RED))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("⚠️ 오류 발생: {err}")).color(COLOR_ACCENT_RED).strong());
                });
        }

        // 추출 완료 결과 카드
        if let Some(ref info) = state.extracted_info {
            accent_card_frame(COLOR_ACCENT_GREEN).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✓ 추출 성공").size(15.0).strong().color(COLOR_ACCENT_GREEN));
                    ui.label(RichText::new(&info.filename).strong().color(COLOR_TEXT_PRIMARY));
                    ui.label(RichText::new(format!("({})", info.duration_formatted)).color(COLOR_TEXT_MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let edit_btn = egui::Button::new(RichText::new("🎛 트랙 편집기로 전송").strong().size(12.0).color(Color32::WHITE))
                            .fill(COLOR_ACCENT_PURPLE)
                            .rounding(Rounding::same(6.0));
                        if ui.add(edit_btn).clicked() {
                            *on_send_to_editor = Some(info.clone());
                        }

                        let save_btn = egui::Button::new(RichText::new("💾 MP3 파일 저장...").size(12.0).color(COLOR_TEXT_PRIMARY))
                            .fill(COLOR_BG_CARD_SUB)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(6.0));
                        if ui.add(save_btn).clicked() {
                            if let Some(save_target) = FileDialog::new()
                                .set_file_name(&info.filename)
                                .add_filter("MP3 Audio", &["mp3"])
                                .save_file()
                            {
                                let _ = std::fs::copy(&info.file_path, &save_target);
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                render_audio_player(ui, &mut state.player, "추출 음원 미리듣기");
            });
        }
    });
}
