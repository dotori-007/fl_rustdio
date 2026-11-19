use egui::{Color32, Frame, Margin, Pos2, Rect, Rounding, Shape, Stroke, Ui, Vec2};
use std::sync::{Arc, Mutex};
use crate::audio::dsp::{VisualizerState, SPECTRUM_BANDS_COUNT};
use super::theme::{
    COLOR_ACCENT_AMBER, COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN, COLOR_ACCENT_PURPLE,
    COLOR_BG_PANEL, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};

#[allow(dead_code)]
pub fn render_bottom_visualizer(
    ui: &mut Ui,
    visualizer: &Arc<Mutex<VisualizerState>>,
    is_playing: bool,
    active_track_name: Option<&str>,
) {
    let mut viz_lock = match visualizer.try_lock() {
        Ok(guard) => guard,
        Err(_) => return,
    };

    if !is_playing {
        viz_lock.update_decay();
    }

    let bars = viz_lock.spectrum_bars;
    let caps = viz_lock.peak_caps;
    let waveform = viz_lock.waveform;
    let rms = viz_lock.rms_level;
    drop(viz_lock);

    Frame::none()
        .fill(COLOR_BG_PANEL)
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .inner_margin(Margin::symmetric(16.0, 8.0))
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let height = 70.0_f32;

            ui.horizontal(|ui| {
                // 좌측: 모니터링 상태 및 트랙 정보
                ui.allocate_ui(Vec2::new(210.0, height), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            let (dot_color, status_text) = if is_playing {
                                (COLOR_ACCENT_GREEN, "DSP LIVE MONITOR")
                            } else {
                                (COLOR_TEXT_MUTED, "DSP STANDBY")
                            };
                            ui.label(egui::RichText::new("●").color(dot_color).size(11.0));
                            ui.label(egui::RichText::new(status_text).strong().size(11.5).color(dot_color));
                        });

                        ui.add_space(2.0);

                        let track_label = active_track_name.unwrap_or("재생 중인 트랙 없음");
                        let truncated = if track_label.chars().count() > 22 {
                            format!("{}...", track_label.chars().take(20).collect::<String>())
                        } else {
                            track_label.to_string()
                        };
                        ui.label(egui::RichText::new(truncated).strong().size(12.0).color(COLOR_TEXT_PRIMARY));

                        ui.add_space(3.0);

                        // RMS 레벨 미터 바
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("LVL").size(10.0).monospace().color(COLOR_TEXT_MUTED));
                            let meter_width = 120.0_f32;
                            let meter_height = 8.0_f32;
                            let (rect, _) = ui.allocate_exact_size(Vec2::new(meter_width, meter_height), egui::Sense::hover());
                            let painter = ui.painter();

                            // 배경 바
                            painter.rect_filled(rect, Rounding::same(3.0), Color32::from_rgb(25, 30, 42));

                            // 활성 레벨 바
                            let fill_w = (meter_width * rms.clamp(0.0, 1.0)).max(2.0);
                            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, meter_height));
                            let fill_color = if rms > 0.85 {
                                COLOR_ACCENT_AMBER
                            } else if rms > 0.6 {
                                COLOR_ACCENT_CYAN
                            } else {
                                COLOR_ACCENT_PURPLE
                            };
                            painter.rect_filled(fill_rect, Rounding::same(3.0), fill_color);
                        });
                    });
                });

                ui.separator();
                ui.add_space(8.0);

                // 중앙 및 우측: 스펙트럼 바 & 오실로스코프 파형 캔버스
                let canvas_width = (available_width - 240.0).max(180.0);
                let (rect, _) = ui.allocate_exact_size(Vec2::new(canvas_width, height), egui::Sense::hover());
                let painter = ui.painter();

                // 1. 캔버스 배경 그리드 라인
                let mid_y = rect.min.y + height * 0.5;
                painter.line_segment(
                    [Pos2::new(rect.min.x, mid_y), Pos2::new(rect.max.x, mid_y)],
                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(40, 50, 70, 60)),
                );

                // 2. 오실로스코프 파형 (Waveform Line)
                let time = ui.input(|i| i.time as f32);
                let mut wave_points = Vec::with_capacity(64);
                let step = (waveform.len() / 64).max(1);

                for i in 0..64 {
                    let sample_idx = (i * step).min(waveform.len() - 1);
                    let raw_sample = if is_playing {
                        waveform[sample_idx]
                    } else {
                        // 아이들 숨쉬는 웨이브 (Idle animation)
                        (time * 2.5 + i as f32 * 0.15).sin() * 0.08
                    };

                    let px = rect.min.x + (i as f32 / 63.0) * canvas_width;
                    let py = mid_y - (raw_sample * height * 0.42).clamp(-height * 0.45, height * 0.45);
                    wave_points.push(Pos2::new(px, py));
                }

                if wave_points.len() >= 2 {
                    let wave_stroke = Stroke::new(
                        1.2_f32,
                        Color32::from_rgba_premultiplied(56, 189, 248, if is_playing { 130 } else { 50 }),
                    );
                    painter.add(Shape::line(wave_points, wave_stroke));
                }

                // 3. 네온 스펙트럼 바 (Spectrum Bars)
                let num_bars = SPECTRUM_BANDS_COUNT;
                let bar_gap = 3.0_f32;
                let total_gaps = (num_bars - 1) as f32 * bar_gap;
                let bar_width = ((canvas_width - total_gaps) / num_bars as f32).max(2.0);

                for i in 0..num_bars {
                    let val = if is_playing {
                        bars[i].clamp(0.0, 1.0)
                    } else {
                        // 대기 모드 은은한 비트 애니메이션
                        ((time * 2.0 + i as f32 * 0.3).sin().abs() * 0.12).max(0.02)
                    };

                    let cap_val = if is_playing {
                        caps[i].clamp(0.0, 1.0)
                    } else {
                        val + 0.03
                    };

                    let bar_h = (val * (height - 10.0)).max(3.0);
                    let bx = rect.min.x + i as f32 * (bar_width + bar_gap);
                    let by = rect.max.y - 4.0 - bar_h;

                    let bar_rect = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(bar_width, bar_h));

                    // 주파수 대역에 따른 네온 그라데이션 색상
                    let bar_color = if i < 8 {
                        // 저음역: 인디고/퍼플
                        COLOR_ACCENT_PURPLE
                    } else if i < 22 {
                        // 중음역: 시안/일렉트릭 스카이
                        COLOR_ACCENT_CYAN
                    } else {
                        // 고음역: 에메랄드 그린
                        COLOR_ACCENT_GREEN
                    };

                    painter.rect_filled(bar_rect, Rounding::same(2.0), bar_color);

                    // 상단 피크 캡 (Peak Cap Line)
                    if cap_val > 0.04 {
                        let cap_y = rect.max.y - 4.0 - (cap_val * (height - 10.0)).max(4.0);
                        let cap_rect = Rect::from_min_size(Pos2::new(bx, cap_y), Vec2::new(bar_width, 1.8));
                        painter.rect_filled(cap_rect, Rounding::same(1.0), Color32::from_rgb(224, 242, 254));
                    }
                }
            });
        });
}
