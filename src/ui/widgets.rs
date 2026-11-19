use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};
use crate::audio::player::AudioPlayer;
use crate::audio::engine::format_duration;
use super::theme::{
    COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN, COLOR_BG_CARD_SUB, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};

pub fn render_audio_player(ui: &mut Ui, player: &mut AudioPlayer, title: &str) {
    Frame::none()
        .fill(COLOR_BG_CARD_SUB)
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::symmetric(10.0, 7.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // 트랙/플레이어 타이틀 라벨
                ui.label(RichText::new(format!("▶ {}", title)).size(12.5).strong().color(COLOR_ACCENT_CYAN));
                ui.add_space(4.0);

                // 재생 / 일시정지 버튼
                let (play_text, btn_fill, btn_color) = if player.is_playing {
                    ("⏸ 일시정지", Color32::from_rgb(180, 83, 9), Color32::WHITE) // 앰버
                } else {
                    ("▶ 재생", COLOR_ACCENT_GREEN, Color32::BLACK) // 에메랄드
                };

                let play_btn = egui::Button::new(RichText::new(play_text).size(12.0).strong().color(btn_color))
                    .fill(btn_fill)
                    .rounding(Rounding::same(5.0));

                if ui.add(play_btn).clicked() {
                    let _ = player.toggle_play();
                }

                // 정지 버튼
                let stop_btn = egui::Button::new(RichText::new("⏹ 정지").size(12.0).color(COLOR_TEXT_PRIMARY))
                    .fill(Color32::from_rgb(45, 54, 72))
                    .rounding(Rounding::same(5.0));
                if ui.add(stop_btn).clicked() {
                    player.stop();
                }

                ui.add_space(6.0);

                // 시간 표시 칩 (현재 / 전체)
                let cur_sec = player.current_position().as_secs_f64();
                let total_sec = player.duration.as_secs_f64();
                let time_str = format!("{:>5} / {:>5}", format_duration(cur_sec), format_duration(total_sec));
                ui.label(RichText::new(time_str).size(12.0).monospace().strong().color(COLOR_TEXT_PRIMARY));

                // 타임라인 슬라이더
                let mut ratio = player.progress_ratio();
                let slider_width = (ui.available_width() - 120.0).max(100.0);
                let slider = egui::Slider::new(&mut ratio, 0.0..=1.0)
                    .show_value(false)
                    .text("");
                
                let response = ui.add_sized([slider_width, 18.0], slider);
                if response.changed() || response.drag_stopped() {
                    let _ = player.seek_to_ratio(ratio);
                }

                // 볼륨 컨트롤
                ui.add_space(4.0);
                ui.label(RichText::new("🔊").size(11.5).color(COLOR_TEXT_MUTED));
                let mut vol_pct = (player.volume * 100.0).round() as u32;
                let vol_slider = egui::Slider::new(&mut vol_pct, 0..=150)
                    .show_value(false);
                if ui.add_sized([55.0, 18.0], vol_slider).changed() {
                    player.set_volume(vol_pct as f32 / 100.0);
                }
                ui.label(RichText::new(format!("{}%", vol_pct)).size(11.0).monospace().color(COLOR_TEXT_MUTED));
            });
        });
}
