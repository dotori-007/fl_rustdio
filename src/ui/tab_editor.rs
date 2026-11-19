use std::path::PathBuf;
use egui::{Color32, Frame, Margin, RichText, Rounding, Stroke, Ui};
use rfd::FileDialog;
use crate::audio::dsp::DspParams;
use crate::audio::engine::{probe_audio_info, AudioFormat, AudioInfo, AudioProcessParams, GenrePreset};
use crate::audio::player::AudioPlayer;
use crate::utils::tools::ToolPaths;
use super::tab_extract::ExtractTabState;
use super::theme::{
    accent_card_frame, modern_card_frame, COLOR_ACCENT_AMBER, COLOR_ACCENT_CYAN, COLOR_ACCENT_GREEN,
    COLOR_ACCENT_PURPLE, COLOR_ACCENT_RED, COLOR_BG_CARD_SUB, COLOR_BORDER,
    COLOR_TEXT_MUTED, COLOR_TEXT_PRIMARY,
};
use super::widgets::render_audio_player;

pub struct EditorTabState {
    pub source_info: Option<AudioInfo>,
    pub source_player: AudioPlayer,
    pub preset: GenrePreset,
    pub speed: f32,
    pub pitch_semitones: f32,
    pub vinyl_link: bool,
    pub bass_gain_db: f32,
    pub treble_gain_db: f32,
    pub reverb_pct: f32,
    pub output_format: AudioFormat,
    pub is_processing: bool,
    pub process_error: Option<String>,
    pub result_info: Option<AudioInfo>,
    pub result_player: AudioPlayer,
}

impl Default for EditorTabState {
    fn default() -> Self {
        Self {
            source_info: None,
            source_player: AudioPlayer::new(),
            preset: GenrePreset::Custom,
            speed: 1.0,
            pitch_semitones: 0.0,
            vinyl_link: false,
            bass_gain_db: 0.0,
            treble_gain_db: 0.0,
            reverb_pct: 0.0,
            output_format: AudioFormat::Mp3,
            is_processing: false,
            process_error: None,
            result_info: None,
            result_player: AudioPlayer::new(),
        }
    }
}

impl EditorTabState {
    pub fn load_file(&mut self, path: PathBuf, tools: &ToolPaths) {
        if let Some(ref ffmpeg) = tools.ffmpeg {
            match probe_audio_info(&path, ffmpeg) {
                Ok(info) => {
                    let _ = self.source_player.load_file(&path, info.duration);
                    self.source_info = Some(info);
                    self.process_error = None;
                    self.sync_realtime_dsp();
                }
                Err(e) => {
                    self.process_error = Some(format!("파일 분석 실패: {e}"));
                }
            }
        } else {
            self.process_error = Some("FFmpeg를 찾을 수 없습니다.".to_string());
        }
    }

    pub fn sync_realtime_dsp(&mut self) {
        let params = DspParams {
            speed: self.speed,
            pitch_semitones: self.pitch_semitones,
            vinyl_link: self.vinyl_link,
            bass_gain_db: self.bass_gain_db,
            treble_gain_db: self.treble_gain_db,
            reverb_pct: self.reverb_pct,
            volume: self.source_player.volume,
        };
        self.source_player.update_dsp_params(&params);
    }

    pub fn apply_preset(&mut self, preset: GenrePreset) {
        self.preset = preset;
        match preset {
            GenrePreset::Custom => {}
            GenrePreset::ClassicNightcore => {
                self.speed = 1.25;
                self.pitch_semitones = 0.0;
                self.vinyl_link = true;
                self.bass_gain_db = 2.0;
                self.treble_gain_db = 2.0;
                self.reverb_pct = 0.0;
            }
            GenrePreset::ModernNightcore => {
                self.speed = 1.20;
                self.pitch_semitones = 3.0;
                self.vinyl_link = false;
                self.bass_gain_db = 4.0;
                self.treble_gain_db = 3.5;
                self.reverb_pct = 10.0;
            }
            GenrePreset::LolicoreHyper => {
                self.speed = 1.45;
                self.pitch_semitones = 5.0;
                self.vinyl_link = false;
                self.bass_gain_db = 5.0;
                self.treble_gain_db = 4.0;
                self.reverb_pct = 0.0;
            }
            GenrePreset::LolicoreExtreme => {
                self.speed = 1.65;
                self.pitch_semitones = 7.0;
                self.vinyl_link = false;
                self.bass_gain_db = 3.0;
                self.treble_gain_db = 5.0;
                self.reverb_pct = 0.0;
            }
            GenrePreset::SlowedReverb => {
                self.speed = 0.85;
                self.pitch_semitones = -3.0;
                self.vinyl_link = false;
                self.bass_gain_db = 2.0;
                self.treble_gain_db = 0.0;
                self.reverb_pct = 25.0;
            }
        }
        self.sync_realtime_dsp();
    }

    pub fn reset_params(&mut self) {
        self.preset = GenrePreset::Custom;
        self.speed = 1.0;
        self.pitch_semitones = 0.0;
        self.vinyl_link = false;
        self.bass_gain_db = 0.0;
        self.treble_gain_db = 0.0;
        self.reverb_pct = 0.0;
        self.output_format = AudioFormat::Mp3;
        self.sync_realtime_dsp();
    }
}

pub fn render_editor_tab(
    ui: &mut Ui,
    state: &mut EditorTabState,
    extract_state: &mut ExtractTabState,
    tools: &ToolPaths,
    tx_process: &crossbeam_channel::Sender<(PathBuf, PathBuf, AudioProcessParams)>,
    tx_extract: &crossbeam_channel::Sender<String>,
) {
    ui.vertical(|ui| {
        // 드래그 앤 드롭 파일 수신
        ui.ctx().input(|i| {
            if !i.raw.dropped_files.is_empty() {
                for file in &i.raw.dropped_files {
                    if let Some(ref path) = file.path {
                        state.load_file(path.clone(), tools);
                        break;
                    }
                }
            }
        });

        // [상단 와이드 카드]: 온라인 URL 음원 추출 (YouTube / SoundCloud 지원)
        modern_card_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🌐 온라인 URL 음원 추출").size(14.0).strong().color(COLOR_ACCENT_PURPLE));
                ui.label(RichText::new("YouTube / SoundCloud 지원").size(11.0).color(COLOR_TEXT_MUTED));

                if extract_state.is_extracting {
                    ui.add_space(8.0);
                    ui.spinner();
                    let msg = extract_state.status_message.as_deref().unwrap_or("다운로드 중...");
                    ui.label(RichText::new(msg).size(11.5).color(COLOR_ACCENT_CYAN).strong());
                }
            });

            ui.add_space(6.0);

            ui.horizontal(|ui| {
                let input_width = (ui.available_width() - 110.0).max(200.0);
                Frame::none()
                    .fill(Color32::from_rgb(18, 23, 33))
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(10.0, 5.0))
                    .show(ui, |ui| {
                        let text_edit = egui::TextEdit::singleline(&mut extract_state.url_input)
                            .hint_text(RichText::new("추출할 YouTube 또는 SoundCloud 음원 URL을 입력하세요...").color(COLOR_TEXT_MUTED))
                            .text_color(Color32::WHITE)
                            .desired_width(input_width)
                            .frame(false);
                        ui.add(text_edit);
                    });

                let can_extract = !extract_state.url_input.trim().is_empty()
                    && !extract_state.is_extracting
                    && tools.ytdlp.is_some()
                    && tools.ffmpeg.is_some();

                let extract_btn = egui::Button::new(RichText::new("⬇ 추출 시작").strong().size(12.0).color(Color32::WHITE))
                    .fill(if can_extract { COLOR_ACCENT_PURPLE } else { COLOR_BG_CARD_SUB })
                    .stroke(Stroke::new(1.0_f32, if can_extract { Color32::from_rgb(167, 139, 250) } else { COLOR_BORDER }))
                    .rounding(Rounding::same(6.0));

                if ui.add_enabled(can_extract, extract_btn).clicked() {
                    extract_state.is_extracting = true;
                    extract_state.error_message = None;
                    extract_state.status_message = Some("음원을 고속 다운로드 및 변환하는 중입니다...".to_string());
                    let _ = tx_extract.send(extract_state.url_input.trim().to_string());
                }
            });

            if let Some(ref err) = extract_state.error_message {
                ui.add_space(4.0);
                ui.label(RichText::new(format!("⚠️ 오류: {err}")).size(11.5).color(COLOR_ACCENT_RED).strong());
            }
        });

        ui.add_space(8.0);

        // 2열 분할 레이아웃 (좌: 원본 소스 / 우: 이펙트 & 프리셋 스튜디오)
        ui.columns(2, |cols| {
            // [좌측 카드]: 원본 트랙 로더 & 분석 정보
            modern_card_frame().show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("📁 원본 오디오 트랙").size(15.0).strong().color(COLOR_ACCENT_CYAN));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let open_btn = egui::Button::new(RichText::new("📂 파일 선택...").size(12.0).strong().color(Color32::WHITE))
                            .fill(COLOR_BG_CARD_SUB)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(6.0));
                        if ui.add(open_btn).clicked() {
                            if let Some(file) = FileDialog::new()
                                .add_filter("Audio Files", &["mp3", "wav", "m4a", "flac", "ogg", "aac", "wma"])
                                .pick_file()
                            {
                                state.load_file(file, tools);
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                if let Some(ref info) = state.source_info {
                    // 파일명 및 포맷 배지
                    Frame::none()
                        .fill(COLOR_BG_CARD_SUB)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🎵").size(16.0));
                                ui.label(RichText::new(&info.filename).strong().size(13.5).color(COLOR_TEXT_PRIMARY));
                            });
                            ui.add_space(4.0);

                            // 메타데이터 칩들
                            ui.horizontal(|ui| {
                                let size_mb = info.size_bytes as f64 / (1024.0 * 1024.0);
                                let chip = |ui: &mut egui::Ui, text: String, color: Color32| {
                                    Frame::none()
                                        .fill(Color32::from_rgb(18, 22, 32))
                                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                                        .rounding(Rounding::same(4.0))
                                        .inner_margin(Margin::symmetric(6.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.label(RichText::new(text).size(11.0).color(color));
                                        });
                                };

                                chip(ui, format!("⏱ {}", info.duration_formatted), COLOR_ACCENT_CYAN);
                                chip(ui, format!("📊 {}", info.bitrate), COLOR_TEXT_MUTED);
                                chip(ui, format!("💾 {:.1} MB", size_mb), COLOR_ACCENT_GREEN);
                            });
                        });

                    ui.add_space(8.0);
                    render_audio_player(ui, &mut state.source_player, "원본 오디오");
                } else {
                    // 미로드 상태 (드래그 앤 드롭 영역)
                    Frame::none()
                        .fill(COLOR_BG_CARD_SUB)
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 78)))
                        .rounding(Rounding::same(8.0))
                        .inner_margin(Margin::same(24.0))
                        .show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(RichText::new("📥").size(34.0));
                                ui.add_space(8.0);
                                ui.label(RichText::new("오디오 파일을 여기에 드래그 앤 드롭하세요").size(13.0).strong().color(COLOR_TEXT_PRIMARY));
                                ui.add_space(2.0);
                                ui.label(RichText::new("MP3, WAV, FLAC, M4A, OGG, AAC 지원").size(11.0).color(COLOR_TEXT_MUTED));
                                ui.add_space(40.0);
                            });
                        });
                }
            });

            // [우측 카드]: 이펙트 조절 & 장르 특화 프리셋
            modern_card_frame().show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚡ 사운드 이펙트 & 프리셋").size(15.0).strong().color(COLOR_ACCENT_PURPLE));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let reset_btn = egui::Button::new(RichText::new("↺ 초기화").size(11.5).color(COLOR_TEXT_MUTED))
                            .fill(COLOR_BG_CARD_SUB)
                            .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                            .rounding(Rounding::same(4.0));
                        if ui.add(reset_btn).clicked() {
                            state.reset_params();
                        }
                    });
                });

                ui.add_space(6.0);

                // 원클릭 장르 프리셋 칩/버튼 바
                ui.horizontal_wrapped(|ui| {
                    for p in GenrePreset::ALL {
                        let is_sel = state.preset == p;
                        let (fill, stroke, text_color) = if is_sel {
                            (COLOR_ACCENT_PURPLE, Stroke::new(1.0_f32, Color32::from_rgb(196, 181, 253)), Color32::WHITE)
                        } else {
                            (COLOR_BG_CARD_SUB, Stroke::new(1.0_f32, COLOR_BORDER), COLOR_TEXT_MUTED)
                        };

                        let btn = egui::Button::new(RichText::new(p.label()).size(11.0).color(text_color).strong())
                            .fill(fill)
                            .stroke(stroke)
                            .rounding(Rounding::same(5.0));

                        if ui.add(btn).clicked() {
                            state.apply_preset(p);
                        }
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);

                // 1. 배속 (Speed)
                ui.horizontal(|ui| {
                    ui.allocate_ui(egui::vec2(130.0, 20.0), |ui| {
                        ui.label(RichText::new("⚡ 재생 배속 (Speed):").size(12.5).color(COLOR_TEXT_PRIMARY));
                    });

                    let mut changed = false;
                    let drag = egui::DragValue::new(&mut state.speed)
                        .speed(0.01)
                        .range(0.25..=4.00)
                        .max_decimals(2)
                        .suffix("x");
                    if ui.add_sized([60.0, 20.0], drag).on_hover_text("클릭하여 직접 수치 입력 가능 (드래그 조절 가능)").changed() {
                        changed = true;
                    }

                    let slider_width = ui.available_width().max(60.0);
                    let slider = egui::Slider::new(&mut state.speed, 0.5..=2.5)
                        .step_by(0.01)
                        .show_value(false);
                    if ui.add_sized([slider_width, 20.0], slider).changed() {
                        changed = true;
                    }

                    if changed {
                        state.preset = GenrePreset::Custom;
                        state.sync_realtime_dsp();
                    }
                });

                // 바이닐 연동 모드 토글
                ui.horizontal(|ui| {
                    let chk = ui.checkbox(
                        &mut state.vinyl_link,
                        RichText::new("🔗 바이닐 연동 가속 (턴테이블 아날로그 방식)").size(11.5).strong().color(COLOR_ACCENT_AMBER)
                    ).on_hover_text("체크 시 턴테이블처럼 속도 증가에 비례하여 피치도 함께 상승합니다.\n체크 해제 시 속도를 올려도 원곡 피치가 완벽하게 보존됩니다.");
                    if chk.changed() {
                        state.preset = GenrePreset::Custom;
                        state.sync_realtime_dsp();
                    }
                });

                ui.add_space(2.0);

                // 2. 피치 (Pitch)
                ui.horizontal(|ui| {
                    ui.allocate_ui(egui::vec2(130.0, 20.0), |ui| {
                        ui.label(RichText::new("🎵 피치 조절 (Pitch):").size(12.5).color(COLOR_TEXT_PRIMARY));
                    });

                    if state.vinyl_link {
                        let approx_semitones = (12.0 * (state.speed.max(0.1).log2())).round() as i32;
                        let text = format!("🔒 바이닐 자동 연동 ({:+} 반음)", approx_semitones);
                        ui.label(RichText::new(text).color(COLOR_TEXT_MUTED).size(11.5));
                    } else {
                        let mut changed = false;
                        let drag = egui::DragValue::new(&mut state.pitch_semitones)
                            .speed(0.5)
                            .range(-24.0..=24.0)
                            .max_decimals(1)
                            .suffix(" 반음");
                        if ui.add_sized([60.0, 20.0], drag).on_hover_text("클릭하여 직접 수치 입력 가능 (-24 ~ +24 반음)").changed() {
                            changed = true;
                        }

                        let slider_width = ui.available_width().max(60.0);
                        let slider = egui::Slider::new(&mut state.pitch_semitones, -12.0..=12.0)
                            .step_by(1.0)
                            .show_value(false);
                        if ui.add_sized([slider_width, 20.0], slider).changed() {
                            changed = true;
                        }

                        if changed {
                            state.preset = GenrePreset::Custom;
                            state.sync_realtime_dsp();
                        }
                    }
                });

                ui.add_space(2.0);

                // 3. 베이스 부스트 (Bass Boost)
                ui.horizontal(|ui| {
                    ui.allocate_ui(egui::vec2(130.0, 20.0), |ui| {
                        ui.label(RichText::new("🔊 베이스 부스트:").size(12.5).color(COLOR_TEXT_PRIMARY));
                    });

                    let mut changed = false;
                    let drag = egui::DragValue::new(&mut state.bass_gain_db)
                        .speed(0.2)
                        .range(0.0..=24.0)
                        .max_decimals(1)
                        .suffix(" dB");
                    if ui.add_sized([60.0, 20.0], drag).on_hover_text("클릭하여 직접 수치 입력 가능 (0.0 ~ 24.0 dB)").changed() {
                        changed = true;
                    }

                    let slider_width = ui.available_width().max(60.0);
                    let slider = egui::Slider::new(&mut state.bass_gain_db, 0.0..=20.0)
                        .step_by(0.5)
                        .show_value(false);
                    if ui.add_sized([slider_width, 20.0], slider).changed() {
                        changed = true;
                    }

                    if changed {
                        state.preset = GenrePreset::Custom;
                        state.sync_realtime_dsp();
                    }
                });

                ui.add_space(2.0);

                // 4. 고음역 에어 / 트레블 부스트
                ui.horizontal(|ui| {
                    ui.allocate_ui(egui::vec2(130.0, 20.0), |ui| {
                        ui.label(RichText::new("💨 고음 에어 / 보컬:").size(12.5).color(COLOR_TEXT_PRIMARY));
                    });

                    let mut changed = false;
                    let drag = egui::DragValue::new(&mut state.treble_gain_db)
                        .speed(0.2)
                        .range(0.0..=20.0)
                        .max_decimals(1)
                        .suffix(" dB");
                    if ui.add_sized([60.0, 20.0], drag).on_hover_text("클릭하여 직접 수치 입력 가능 (0.0 ~ 20.0 dB)").changed() {
                        changed = true;
                    }

                    let slider_width = ui.available_width().max(60.0);
                    let slider = egui::Slider::new(&mut state.treble_gain_db, 0.0..=15.0)
                        .step_by(0.5)
                        .show_value(false);
                    if ui.add_sized([slider_width, 20.0], slider).changed() {
                        changed = true;
                    }

                    if changed {
                        state.preset = GenrePreset::Custom;
                        state.sync_realtime_dsp();
                    }
                });

                ui.add_space(2.0);

                // 5. 공간감 리버브
                ui.horizontal(|ui| {
                    ui.allocate_ui(egui::vec2(130.0, 20.0), |ui| {
                        ui.label(RichText::new("🌌 공간감 리버브:").size(12.5).color(COLOR_TEXT_PRIMARY));
                    });

                    let mut changed = false;
                    let drag = egui::DragValue::new(&mut state.reverb_pct)
                        .speed(1.0)
                        .range(0.0..=100.0)
                        .max_decimals(0)
                        .suffix("%");
                    if ui.add_sized([60.0, 20.0], drag).on_hover_text("클릭하여 직접 수치 입력 가능 (0 ~ 100%)").changed() {
                        changed = true;
                    }

                    let slider_width = ui.available_width().max(60.0);
                    let slider = egui::Slider::new(&mut state.reverb_pct, 0.0..=50.0)
                        .step_by(1.0)
                        .show_value(false);
                    if ui.add_sized([slider_width, 20.0], slider).changed() {
                        changed = true;
                    }

                    if changed {
                        state.preset = GenrePreset::Custom;
                        state.sync_realtime_dsp();
                    }
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // 하단 내보내기 포맷 & 실행 액션 바
                ui.horizontal(|ui| {
                    ui.label(RichText::new("내보내기:").size(12.0).color(COLOR_TEXT_MUTED));
                    egui::ComboBox::from_id_salt("editor_format_combo")
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
                        let can_process = state.source_info.is_some() && !state.is_processing && tools.ffmpeg.is_some();
                        let apply_btn = egui::Button::new(RichText::new("✨ 이펙트 적용 & 렌더링").strong().size(12.5).color(Color32::WHITE))
                            .fill(if can_process { COLOR_ACCENT_PURPLE } else { COLOR_BG_CARD_SUB })
                            .stroke(Stroke::new(1.0_f32, if can_process { Color32::from_rgb(167, 139, 250) } else { COLOR_BORDER }))
                            .rounding(Rounding::same(6.0));

                        if ui.add_enabled(can_process, apply_btn).clicked() {
                            if let Some(ref source) = state.source_info {
                                state.is_processing = true;
                                state.process_error = None;

                                let clean_id = &uuid::Uuid::new_v4().to_string()[..8];
                                let stem = std::path::Path::new(&source.filename)
                                    .file_stem()
                                    .map(|s| s.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "audio".to_string());
                                let ext = state.output_format.extension();
                                let out_name = format!("edited_{clean_id}_{stem}.{ext}");
                                let out_path = crate::utils::paths::get_outputs_dir().join(out_name);

                                let params = AudioProcessParams {
                                    speed: state.speed,
                                    pitch_semitones: state.pitch_semitones,
                                    vinyl_link: state.vinyl_link,
                                    bass_gain_db: state.bass_gain_db,
                                    treble_gain_db: state.treble_gain_db,
                                    reverb_pct: state.reverb_pct,
                                    format: state.output_format,
                                };

                                let _ = tx_process.send((
                                    source.file_path.clone(),
                                    out_path,
                                    params,
                                ));
                            }
                        }
                    });
                });
            });
        });

        ui.add_space(10.0);

        // 진행 중 상태 표시
        if state.is_processing {
            Frame::none()
                .fill(COLOR_BG_CARD_SUB)
                .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_CYAN))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        let fmt_label = state.output_format.label();
                        ui.label(RichText::new(format!("{fmt_label} 포맷으로 오디오를 고속 렌더링하고 있습니다...")).strong().color(COLOR_ACCENT_CYAN));
                    });
                });
        }

        // 에러 배너
        if let Some(ref err) = state.process_error {
            Frame::none()
                .fill(Color32::from_rgb(38, 18, 22))
                .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_RED))
                .rounding(Rounding::same(8.0))
                .inner_margin(Margin::same(12.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(format!("⚠️ 오류 발생: {err}")).color(COLOR_ACCENT_RED).strong());
                });
        }

        // 결과 카드 (에메랄드 그린 강조)
        if let Some(ref result) = state.result_info {
            accent_card_frame(COLOR_ACCENT_GREEN).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("✓ 렌더링 완료").size(15.0).strong().color(COLOR_ACCENT_GREEN));
                    ui.label(RichText::new(&result.filename).strong().color(COLOR_TEXT_PRIMARY));
                    ui.label(RichText::new(format!("({})", result.duration_formatted)).color(COLOR_TEXT_MUTED));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn_text = format!("💾 {} 파일로 저장...", state.output_format.extension().to_uppercase());
                        let save_btn = egui::Button::new(RichText::new(btn_text).strong().size(12.5).color(Color32::WHITE))
                            .fill(Color32::from_rgb(5, 150, 105)) // 에메랄드
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
                                if let Err(e) = std::fs::copy(&result.file_path, &save_target) {
                                    state.process_error = Some(format!("파일 저장 실패: {e}"));
                                }
                            }
                        }
                    });
                });

                ui.add_space(8.0);
                render_audio_player(ui, &mut state.result_player, "결과 미리듣기");
            });
        }
    });
}
