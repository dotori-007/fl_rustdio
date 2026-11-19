use std::path::PathBuf;
use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};
use rfd::FileDialog;
use crate::audio::engine::{probe_audio_info, AudioFormat, AudioInfo};
use crate::audio::player::AudioPlayer;
use crate::utils::tools::ToolPaths;
use super::theme::{
    accent_card_frame, modern_card_frame, COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN,
    COLOR_ACCENT_RED, COLOR_BG_CARD_SUB, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use super::widgets::render_audio_player;

pub struct MergeTabState {
    pub files: Vec<AudioInfo>,
    pub output_format: AudioFormat,
    pub is_merging: bool,
    pub error_message: Option<String>,
    pub result_info: Option<AudioInfo>,
    pub result_player: AudioPlayer,
}

impl Default for MergeTabState {
    fn default() -> Self {
        Self {
            files: Vec::new(),
            output_format: AudioFormat::Mp3,
            is_merging: false,
            error_message: None,
            result_info: None,
            result_player: AudioPlayer::new(),
        }
    }
}

impl MergeTabState {
    pub fn add_files(&mut self, paths: Vec<PathBuf>, tools: &ToolPaths) {
        if let Some(ref ffmpeg) = tools.ffmpeg {
            for path in paths {
                if let Ok(info) = probe_audio_info(&path, ffmpeg) {
                    self.files.push(info);
                }
            }
        }
    }
}

pub fn render_merge_tab(
    ui: &mut Ui,
    state: &mut MergeTabState,
    tools: &ToolPaths,
    tx_merge: &crossbeam_channel::Sender<(Vec<PathBuf>, PathBuf, AudioFormat)>,
) {
    ui.vertical(|ui| {
        // 상단 제어 바 카드
        modern_card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔗 다중 오디오 트랙 병합").size(15.0).strong().color(COLOR_ACCENT_CYAN));
                
                // 등록된 파일 개수 칩
                Frame::none()
                    .fill(COLOR_BG_CARD_SUB)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(4.0))
                    .inner_margin(Margin::symmetric(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(format!("총 {}개", state.files.len())).size(11.5).color(COLOR_TEXT_MUTED));
                    });

                let add_btn = egui::Button::new(RichText::new("➕ 파일 추가...").size(12.0).strong().color(Color32::WHITE))
                    .fill(COLOR_BG_CARD_SUB)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(6.0));
                if ui.add(add_btn).clicked() {
                    if let Some(picked) = FileDialog::new()
                        .add_filter("Audio Files", &["mp3", "wav", "m4a", "flac", "ogg"])
                        .pick_files()
                    {
                        state.add_files(picked, tools);
                    }
                }

                if !state.files.is_empty() {
                    let clear_btn = egui::Button::new(RichText::new("목록 비우기").size(11.5).color(COLOR_TEXT_MUTED))
                        .fill(COLOR_BG_CARD_SUB)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(4.0));
                    if ui.add(clear_btn).clicked() {
                        state.files.clear();
                    }
                }

                ui.add_space(8.0);

                // 출력 포맷 선택
                ui.label(RichText::new("출력:").size(12.0).color(COLOR_TEXT_MUTED));
                egui::ComboBox::from_id_salt("merge_format_combo")
                    .selected_text(state.output_format.label())
                    .show_ui(ui, |ui| {
                        for fmt in AudioFormat::ALL {
                            ui.selectable_value(&mut state.output_format, fmt, fmt.label());
                        }
                    });

                if state.output_format.is_lossless() {
                    ui.label(RichText::new("★ 무손실").color(COLOR_ACCENT_GREEN).size(11.0).strong());
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_merge = state.files.len() >= 2 && !state.is_merging && tools.ffmpeg.is_some();
                    let merge_btn = egui::Button::new(RichText::new("🔗 트랙 병합 시작").strong().size(12.5).color(Color32::WHITE))
                        .fill(if can_merge { COLOR_ACCENT_CYAN } else { COLOR_BG_CARD_SUB })
                        .stroke(Stroke::new(1.0_f32, if can_merge { Color32::from_rgb(125, 211, 252) } else { COLOR_BORDER }))
                        .rounding(Rounding::same(6.0));

                    if ui.add_enabled(can_merge, merge_btn).clicked() {
                        state.is_merging = true;
                        state.error_message = None;

                        let clean_id = &uuid::Uuid::new_v4().to_string()[..8];
                        let ext = state.output_format.extension();
                        let out_name = format!("merged_{clean_id}_combined.{ext}");
                        let out_path = crate::utils::paths::get_outputs_dir().join(out_name);

                        let input_paths: Vec<PathBuf> = state.files.iter().map(|f| f.file_path.clone()).collect();
                        let _ = tx_merge.send((input_paths, out_path, state.output_format));
                    }
                });
            });
        });

        ui.add_space(10.0);

        // 파일 목록 리스트 카드
        modern_card_frame().show(ui, |ui| {
            if state.files.is_empty() {
                Frame::none()
                    .fill(COLOR_BG_CARD_SUB)
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 78)))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::same(24.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.add_space(10.0);
                            ui.label(RichText::new("📑").size(32.0));
                            ui.add_space(6.0);
                            ui.label(RichText::new("병합할 오디오 파일 2개 이상을 등록하세요").size(13.0).strong().color(COLOR_TEXT_PRIMARY));
                            ui.label(RichText::new("상단의 '➕ 파일 추가...' 버튼을 눌러 선택할 수 있습니다").size(11.5).color(COLOR_TEXT_MUTED));
                            ui.add_space(10.0);
                        });
                    });
            } else {
                let mut to_remove = None;
                let mut swap_pair = None;

                egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                    for (idx, file) in state.files.iter().enumerate() {
                        Frame::none()
                            .fill(COLOR_BG_CARD_SUB)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(Margin::symmetric(10.0, 7.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    // 순번 뱃지
                                    ui.label(RichText::new(format!("{:02}", idx + 1)).strong().monospace().color(COLOR_ACCENT_CYAN));
                                    ui.add_space(4.0);

                                    // 순서 이동 버튼
                                    if ui.add_enabled(idx > 0, egui::Button::new("▲").small()).clicked() {
                                        swap_pair = Some((idx, idx - 1));
                                    }
                                    if ui.add_enabled(idx + 1 < state.files.len(), egui::Button::new("▼").small()).clicked() {
                                        swap_pair = Some((idx, idx + 1));
                                    }

                                    ui.add_space(6.0);
                                    ui.label(RichText::new(&file.filename).strong().color(COLOR_TEXT_PRIMARY));
                                    ui.label(RichText::new(format!("({})", file.duration_formatted)).size(11.5).color(COLOR_TEXT_MUTED));

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let del_btn = egui::Button::new(RichText::new("✕").size(12.0).color(COLOR_ACCENT_RED))
                                            .fill(Color32::from_rgb(45, 25, 30))
                                            .rounding(Rounding::same(4.0));
                                        if ui.add(del_btn).clicked() {
                                            to_remove = Some(idx);
                                        }
                                    });
                                });
                            });
                        ui.add_space(4.0);
                    }
                });

                if let Some((i, j)) = swap_pair {
                    state.files.swap(i, j);
                }
                if let Some(idx) = to_remove {
                    state.files.remove(idx);
                }
            }
        });

        ui.add_space(10.0);

        // 병합 진행 중 상태 표시
        if state.is_merging {
            Frame::none()
                .fill(COLOR_BG_CARD_SUB)
                .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_CYAN))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        let fmt_label = state.output_format.label();
                        ui.label(RichText::new(format!("{fmt_label} 포맷으로 트랙들을 병합 및 인코딩하고 있습니다...")).strong().color(COLOR_ACCENT_CYAN));
                    });
                });
        }

        // 에러 배너
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

        // 병합 완료 결과 카드
        if let Some(ref result) = state.result_info {
            accent_card_frame(COLOR_ACCENT_GREEN).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✓ 병합 완료").size(15.0).strong().color(COLOR_ACCENT_GREEN));
                    ui.label(RichText::new(&result.filename).strong().color(COLOR_TEXT_PRIMARY));
                    ui.label(RichText::new(format!("({})", result.duration_formatted)).color(COLOR_TEXT_MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = format!("💾 {} 파일로 저장...", state.output_format.extension().to_uppercase());
                        let save_btn = egui::Button::new(RichText::new(btn_text).strong().size(12.5).color(Color32::WHITE))
                            .fill(Color32::from_rgb(5, 150, 105))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(52, 211, 153)))
                            .rounding(Rounding::same(6.0));

                        if ui.add(save_btn).clicked() {
                            let ext = state.output_format.extension();
                            let filter_label = format!("{} Audio", ext.to_uppercase());
                            if let Some(save_target) = FileDialog::new()
                                .set_file_name(&result.filename)
                                .add_filter(&filter_label, &[ext])
                                .save_file()
                            {
                                let _ = std::fs::copy(&result.file_path, &save_target);
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                render_audio_player(ui, &mut state.result_player, "병합 음원 미리듣기");
            });
        }
    });
}
