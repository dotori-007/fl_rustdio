use egui::{Color32, Frame, Margin, Pos2, Rect, Rounding, Stroke, Ui, Vec2};
use crate::audio::engine::format_duration;
use crate::audio::player::AudioPlayer;
use super::theme::{
    COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN, COLOR_BG_CARD_SUB,
    COLOR_BG_PANEL, COLOR_BORDER, COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};

pub fn render_bottom_waveform(
    ui: &mut Ui,
    player: &mut AudioPlayer,
    track_name: Option<&str>,
) {
    Frame::none()
        .fill(COLOR_BG_PANEL)
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .inner_margin(Margin::symmetric(14.0, 8.0))
        .show(ui, |ui| {
            let available_width = ui.available_width();
            let height = 74.0_f32;

            ui.horizontal(|ui| {
                // [좌측 컨트롤러 도크]: 재생/정지, 타임코드, 파일명, 볼륨
                ui.allocate_ui(Vec2::new(230.0, height), |ui| {
                    ui.vertical(|ui| {
                        // 1. 재생 / 정지 버튼 & 타임코드 행
                        ui.horizontal(|ui| {
                            let (play_text, btn_fill, btn_color) = if player.is_playing {
                                ("⏸ 일시정지", Color32::from_rgb(180, 83, 9), Color32::WHITE)
                            } else {
                                ("▶ 재생", COLOR_ACCENT_GREEN, Color32::BLACK)
                            };

                            let play_btn = egui::Button::new(
                                egui::RichText::new(play_text).size(12.0).strong().color(btn_color),
                            )
                            .fill(btn_fill)
                            .rounding(Rounding::same(5.0));

                            let play_resp = ui.add(play_btn).on_hover_text("재생 / 일시정지 (단축키: Space)");
                            if play_resp.clicked() {
                                let _ = player.toggle_play();
                            }

                            let stop_btn = egui::Button::new(
                                egui::RichText::new("⏹").size(12.0).color(COLOR_TEXT_PRIMARY),
                            )
                            .fill(COLOR_BG_CARD_SUB)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(5.0));

                            if ui.add(stop_btn).on_hover_text("정지").clicked() {
                                player.stop();
                            }

                            // 시간 표시 (현재 / 전체)
                            let cur_sec = player.current_position().as_secs_f64();
                            let total_sec = player.duration.as_secs_f64();
                            let time_str = format!("{}/{}", format_duration(cur_sec), format_duration(total_sec));
                            ui.label(
                                egui::RichText::new(time_str)
                                    .size(11.5)
                                    .monospace()
                                    .strong()
                                    .color(COLOR_TEXT_PRIMARY),
                            );
                        });

                        ui.add_space(4.0);

                        // 2. 현재 트랙 이름 라벨
                        let title = track_name.unwrap_or("재생 대기 중");
                        let truncated = if title.chars().count() > 24 {
                            format!("{}...", title.chars().take(22).collect::<String>())
                        } else {
                            title.to_string()
                        };
                        ui.label(
                            egui::RichText::new(truncated)
                                .size(12.0)
                                .strong()
                                .color(COLOR_ACCENT_CYAN),
                        );

                        ui.add_space(3.0);

                        // 3. 볼륨 슬라이더
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("🔊").size(11.0).color(COLOR_TEXT_MUTED));
                            let mut vol_pct = (player.volume * 100.0).round() as u32;
                            if ui.add_sized([65.0, 16.0], egui::Slider::new(&mut vol_pct, 0..=150).show_value(false)).changed() {
                                player.set_volume(vol_pct as f32 / 100.0);
                            }
                            ui.label(
                                egui::RichText::new(format!("{}%", vol_pct))
                                    .size(10.5)
                                    .monospace()
                                    .color(COLOR_TEXT_MUTED),
                            );
                        });
                    });
                });

                ui.separator();
                ui.add_space(8.0);

                // [중앙 & 우측 메인 파형 타임라인 (대화형 인터랙티브 파형)]
                let canvas_width = (available_width - 260.0).max(200.0);
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(canvas_width, height),
                    egui::Sense::click_and_drag(),
                );
                let painter = ui.painter();

                // 파형 캔버스 배경 채우기
                painter.rect_filled(
                    rect,
                    Rounding::same(6.0),
                    Color32::from_rgb(14, 18, 26),
                );
                painter.rect_stroke(
                    rect,
                    Rounding::same(6.0),
                    Stroke::new(1.0_f32, Color32::from_rgb(32, 40, 56)),
                );

                // 유효 파형 렌더링 영역 (내부 패딩 적용)
                let pad_x = 8.0_f32;
                let wave_min_x = rect.min.x + pad_x;
                let wave_max_x = rect.max.x - pad_x;
                let wave_width = (wave_max_x - wave_min_x).max(10.0);

                // 마우스 클릭 또는 드래그 시 해당 위치로 정확히 탐색(Seek)
                if response.clicked() || response.dragged() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        let click_ratio = ((mouse_pos.x - wave_min_x) / wave_width).clamp(0.0, 1.0);
                        let _ = player.seek_to_ratio(click_ratio);
                    }
                }

                let current_ratio = player.progress_ratio();
                let mid_y = rect.min.y + height * 0.5;

                // 재생 헤드의 정확한 화면상 X좌표
                let playhead_x = (wave_min_x + current_ratio * wave_width).clamp(wave_min_x, wave_max_x);

                if player.waveform_peaks.is_empty() {
                    // 음원 미로드 시 가이드 안내
                    painter.line_segment(
                        [Pos2::new(wave_min_x, mid_y), Pos2::new(wave_max_x, mid_y)],
                        Stroke::new(1.0_f32, Color32::from_rgb(35, 45, 62)),
                    );

                    let msg = "오디오 파일을 불러오면 전체 음 파형이 표시되며, 클릭하여 원하는 구간으로 탐색할 수 있습니다 (Space: 재생/일시정지)";
                    painter.text(
                        Pos2::new(rect.center().x, mid_y),
                        egui::Align2::CENTER_CENTER,
                        msg,
                        egui::FontId::proportional(12.0),
                        COLOR_TEXT_MUTED,
                    );
                } else {
                    let peaks = &player.waveform_peaks;
                    let num_bars = peaks.len();

                    // 각 바의 슬롯 폭을 유효 너비에 균등 배분 (너비 오차 0%)
                    let slot_w = wave_width / num_bars as f32;
                    let bar_gap = (slot_w * 0.28).clamp(0.8, 2.0);
                    let bar_w = (slot_w - bar_gap).max(1.0);

                    // 파형 바 렌더링 (SoundCloud 스타일 상하 대칭 미러드 바)
                    for (i, &peak) in peaks.iter().enumerate() {
                        let bx = wave_min_x + i as f32 * slot_w;
                        let bar_center_x = bx + bar_w * 0.5;

                        // 파형 높이 (위아래로 퍼지는 진폭)
                        let bar_half_h = (peak * (height * 0.44)).max(2.0);
                        let bar_rect = Rect::from_min_max(
                            Pos2::new(bx, mid_y - bar_half_h),
                            Pos2::new(bx + bar_w, mid_y + bar_half_h),
                        );

                        // [중요 수정]: 색상 판정을 실제 화면상 재생바(playhead_x) 위치 기준으로 1:1 일치!
                        // 바의 중심이 재생바(playhead_x)를 지나갔을 때만 활성화하여 재생바 앞지름 버그 원천 차단!
                        let bar_color = if bar_center_x <= playhead_x {
                            // 재생된 구간: 네온 시안 -> 퍼플 그라데이션
                            let t = if playhead_x > wave_min_x {
                                ((bar_center_x - wave_min_x) / (playhead_x - wave_min_x)).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            Color32::from_rgb(
                                (56.0 + t * 83.0) as u8,   // 56 -> 139
                                (189.0 - t * 97.0) as u8,  // 189 -> 92
                                (248.0 + t * 8.0) as u8,   // 248 -> 246
                            )
                        } else {
                            // 아직 재생되지 않은 구간: 다크 슬레이트 블루
                            Color32::from_rgb(45, 55, 75)
                        };

                        painter.rect_filled(bar_rect, Rounding::same(1.0), bar_color);
                    }

                    // [재생 헤드 (Playhead Vertical Cursor Line)]
                    // 재생선 발광(Glow) 효과
                    painter.line_segment(
                        [Pos2::new(playhead_x, rect.min.y + 4.0), Pos2::new(playhead_x, rect.max.y - 4.0)],
                        Stroke::new(3.0_f32, Color32::from_rgba_premultiplied(56, 189, 248, 90)),
                    );
                    // 중앙 샤프 화이트 라인
                    painter.line_segment(
                        [Pos2::new(playhead_x, rect.min.y + 4.0), Pos2::new(playhead_x, rect.max.y - 4.0)],
                        Stroke::new(1.8_f32, Color32::WHITE),
                    );

                    // 상단 플레이헤드 인디케이터 핀
                    let pin_rect = Rect::from_center_size(Pos2::new(playhead_x, rect.min.y + 5.0), Vec2::new(6.0, 6.0));
                    painter.rect_filled(pin_rect, Rounding::same(3.0), COLOR_ACCENT_CYAN);

                    // [마우스 호버 시 가이드라인 및 툴팁]
                    if let Some(hover_pos) = response.hover_pos() {
                        let hover_ratio = ((hover_pos.x - wave_min_x) / wave_width).clamp(0.0, 1.0);
                        let hover_secs = player.duration.as_secs_f64() * hover_ratio as f64;
                        let hover_x = hover_pos.x.clamp(wave_min_x, wave_max_x);

                        // 호버 얇은 라인
                        painter.line_segment(
                            [Pos2::new(hover_x, rect.min.y + 4.0), Pos2::new(hover_x, rect.max.y - 4.0)],
                            Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(255, 255, 255, 120)),
                        );

                        // 시간 툴팁 박스
                        let tip_text = format_duration(hover_secs);
                        let tip_pos = Pos2::new(hover_x, rect.min.y + 14.0);
                        painter.rect_filled(
                            Rect::from_center_size(tip_pos, Vec2::new(38.0, 16.0)),
                            Rounding::same(3.0),
                            Color32::from_rgba_premultiplied(20, 24, 35, 220),
                        );
                        painter.text(
                            tip_pos,
                            egui::Align2::CENTER_CENTER,
                            tip_text,
                            egui::FontId::monospace(10.0),
                            Color32::WHITE,
                        );
                    }
                }
            });
        });
}
