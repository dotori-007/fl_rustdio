use egui::{Color32, FontData, FontDefinitions, FontFamily, Frame, Margin, Rounding, Stroke, Visuals};
use std::fs;
use std::path::Path;

// Modern Studio Deep Dark Palette
pub const COLOR_BG_BASE: Color32 = Color32::from_rgb(11, 13, 19);        // #0B0D13 앱 메인 배경
pub const COLOR_BG_PANEL: Color32 = Color32::from_rgb(15, 18, 26);       // #0F121A 패널 배경
pub const COLOR_BG_CARD: Color32 = Color32::from_rgb(22, 27, 38);        // #161B26 카드 표면
pub const COLOR_BG_CARD_SUB: Color32 = Color32::from_rgb(28, 35, 49);    // #1C2331 서브 카드/입력창
pub const COLOR_BG_HOVER: Color32 = Color32::from_rgb(37, 45, 63);       // #252D3F 호버

pub const COLOR_BORDER: Color32 = Color32::from_rgb(38, 46, 64);         // #262E40 기본 경계선

pub const COLOR_ACCENT_PURPLE: Color32 = Color32::from_rgb(139, 92, 246);// #8B5CF6 메인 퍼플 (작업, 프리셋)
pub const COLOR_ACCENT_CYAN: Color32 = Color32::from_rgb(56, 189, 248);   // #38BDF8 스카이/시안 (오디오, 템포)
pub const COLOR_ACCENT_GREEN: Color32 = Color32::from_rgb(16, 185, 129);  // #10B981 에메랄드 그린 (성공, 무손실, 재생)
pub const COLOR_ACCENT_AMBER: Color32 = Color32::from_rgb(245, 158, 11);  // #F59E0B 주의/경고
pub const COLOR_ACCENT_RED: Color32 = Color32::from_rgb(239, 68, 68);     // #EF4444 에러/정지

pub const COLOR_TEXT_PRIMARY: Color32 = Color32::from_rgb(243, 244, 246);// #F3F4F6 주 텍스트
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(156, 163, 175);  // #9CA3AF 보조 텍스트

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // 1. 한국어 맑은 고딕 폰트 로드
    let candidate_fonts = [
        r"C:\Windows\Fonts\malgun.ttf",   // 맑은 고딕
        r"C:\Windows\Fonts\malgunbd.ttf",
        r"C:\Windows\Fonts\gulim.ttc",    // 굴림
        r"C:\Windows\Fonts\batang.ttc",
    ];

    for font_path in candidate_fonts {
        if Path::new(font_path).exists() {
            if let Ok(font_bytes) = fs::read(font_path) {
                let font_name = "korean_font".to_string();
                fonts.font_data.insert(
                    font_name.clone(),
                    FontData::from_owned(font_bytes),
                );

                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, font_name.clone());

                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push(font_name);

                break;
            }
        }
    }

    // 2. 이모지 및 특수 기호 폰트 (Segoe UI Emoji) 폴백 등록 (네모 박스 '□' 깨짐 방지)
    let emoji_candidates = [
        r"C:\Windows\Fonts\seguiemj.ttf", // Windows Segoe UI Emoji
        r"C:\Windows\Fonts\seguisym.ttf", // Segoe UI Symbol
    ];

    for font_path in emoji_candidates {
        if Path::new(font_path).exists() {
            if let Ok(font_bytes) = fs::read(font_path) {
                let font_name = "emoji_font".to_string();
                fonts.font_data.insert(
                    font_name.clone(),
                    FontData::from_owned(font_bytes),
                );

                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .push(font_name.clone());

                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push(font_name);

                break;
            }
        }
    }

    ctx.set_fonts(fonts);
}

pub fn apply_dark_studio_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // 메인 배경 및 패널 색상
    visuals.panel_fill = COLOR_BG_BASE;
    visuals.window_fill = COLOR_BG_CARD;
    visuals.extreme_bg_color = Color32::from_rgb(8, 10, 15);
    visuals.faint_bg_color = COLOR_BG_CARD;

    // 비상호작용 위젯
    visuals.widgets.noninteractive.bg_fill = COLOR_BG_CARD;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_PRIMARY);
    visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

    // 기본(Inactive) 상태 위젯 (버튼, 콤보박스 등)
    visuals.widgets.inactive.bg_fill = COLOR_BG_CARD_SUB;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(226, 232, 240));
    visuals.widgets.inactive.rounding = Rounding::same(6.0);

    // 마우스 호버(Hovered) 상태
    visuals.widgets.hovered.bg_fill = COLOR_BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.2_f32, COLOR_ACCENT_CYAN);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, Color32::WHITE);
    visuals.widgets.hovered.rounding = Rounding::same(6.0);

    // 클릭/활성(Active) 상태
    visuals.widgets.active.bg_fill = COLOR_ACCENT_PURPLE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::WHITE);
    visuals.widgets.active.rounding = Rounding::same(6.0);

    // 텍스트 선택
    visuals.selection.bg_fill = Color32::from_rgb(109, 40, 217);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

    // 윈도우 섀도우 제거 및 모서리 곡률 설정
    visuals.window_rounding = Rounding::same(10.0);
    visuals.window_stroke = Stroke::new(1.0_f32, COLOR_BORDER);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.slider_width = 160.0;
    ctx.set_style(style);
}

/// 모던 카드 컨테이너 프레임
pub fn modern_card_frame() -> Frame {
    Frame::none()
        .fill(COLOR_BG_CARD)
        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(14.0))
}

/// 액센트 강조 카드 프레임 (결과 카드 등)
pub fn accent_card_frame(accent_color: Color32) -> Frame {
    Frame::none()
        .fill(COLOR_BG_CARD)
        .stroke(Stroke::new(1.5_f32, accent_color))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::same(14.0))
}
