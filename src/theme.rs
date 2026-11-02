//! Windows 11 palette, typography and egui style.

use egui::{Color32, FontFamily, FontId, Rounding, Stroke, TextStyle};

#[derive(Clone, Copy)]
pub struct Palette {
    pub window: Color32,
    pub surface: Color32,
    pub surface_alt: Color32,
    pub text: Color32,
    pub text_weak: Color32,
    pub text_disabled: Color32,
    pub stroke: Color32,
    pub stroke_strong: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub accent: Color32,
    pub accent_text: Color32,
    pub selection: Color32,
    pub tab_hover: Color32,
    pub menu_fill: Color32,
    pub menu_stroke: Color32,
    pub close_hover: Color32,
}

impl Palette {
    pub const fn light() -> Self {
        Self {
            window: Color32::from_rgb(0xF3, 0xF3, 0xF3),
            surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            surface_alt: Color32::from_rgb(0xFA, 0xFA, 0xFA),
            text: Color32::from_rgb(0x1A, 0x1A, 0x1A),
            text_weak: Color32::from_rgb(0x5D, 0x5D, 0x5D),
            text_disabled: Color32::from_rgb(0x9A, 0x9A, 0x9A),
            stroke: Color32::from_rgba_premultiplied(0, 0, 0, 0x14),
            stroke_strong: Color32::from_rgba_premultiplied(0, 0, 0, 0x24),
            hover: Color32::from_rgba_premultiplied(0, 0, 0, 0x0A),
            pressed: Color32::from_rgba_premultiplied(0, 0, 0, 0x14),
            accent: Color32::from_rgb(0x00, 0x67, 0xC0),
            accent_text: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            selection: Color32::from_rgba_premultiplied(0x00, 0x78, 0xD4, 0x40),
            tab_hover: Color32::from_rgba_premultiplied(0, 0, 0, 0x08),
            menu_fill: Color32::from_rgb(0xF9, 0xF9, 0xF9),
            menu_stroke: Color32::from_rgba_premultiplied(0, 0, 0, 0x1A),
            close_hover: Color32::from_rgb(0xC4, 0x2B, 0x1C),
        }
    }

    pub const fn dark() -> Self {
        Self {
            window: Color32::from_rgb(0x20, 0x20, 0x20),
            surface: Color32::from_rgb(0x27, 0x27, 0x27),
            surface_alt: Color32::from_rgb(0x2D, 0x2D, 0x2D),
            text: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            text_weak: Color32::from_rgb(0xC5, 0xC5, 0xC5),
            text_disabled: Color32::from_rgb(0x7A, 0x7A, 0x7A),
            stroke: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 0x17),
            stroke_strong: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 0x2A),
            hover: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 0x0C),
            pressed: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 0x17),
            accent: Color32::from_rgb(0x60, 0xCD, 0xFF),
            accent_text: Color32::from_rgb(0x00, 0x1A, 0x2A),
            selection: Color32::from_rgba_premultiplied(0x60, 0xCD, 0xFF, 0x4A),
            tab_hover: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 0x09),
            menu_fill: Color32::from_rgb(0x2B, 0x2B, 0x2B),
            menu_stroke: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 0x1A),
            close_hover: Color32::from_rgb(0xC4, 0x2B, 0x1C),
        }
    }
}

pub const UI_FONT: &str = "selawik";
pub const UI_FONT_SEMIBOLD: &str = "selawik-semibold";
pub const MONO_FONT: &str = "dejavu-mono";
pub const FALLBACK_FONT: &str = "dejavu-sans";

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        UI_FONT.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/selawk.ttf")).into(),
    );
    fonts.font_data.insert(
        UI_FONT_SEMIBOLD.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/selawksb.ttf")).into(),
    );
    fonts.font_data.insert(
        FALLBACK_FONT.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/DejaVuSans.ttf")).into(),
    );
    fonts.font_data.insert(
        MONO_FONT.to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/DejaVuSansMono.ttf")).into(),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, UI_FONT.to_owned());
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(1, FALLBACK_FONT.to_owned());
    let semibold: FontFamily = FontFamily::Name(UI_FONT_SEMIBOLD.into());
    fonts
        .families
        .entry(semibold)
        .or_default()
        .insert(0, UI_FONT_SEMIBOLD.to_owned());

    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, MONO_FONT.to_owned());

    ctx.set_fonts(fonts);
}

pub fn style(palette: Palette, dark: bool) -> egui::Style {
    let mut style = egui::Style::default();

    style.text_styles = [
        (TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
    ]
    .into();

    let v = &mut style.visuals;
    v.dark_mode = dark;
    v.override_text_color = Some(palette.text);
    v.panel_fill = palette.window;
    v.window_fill = palette.menu_fill;
    v.extreme_bg_color = palette.surface;
    v.faint_bg_color = palette.surface_alt;
    v.window_stroke = Stroke::new(1.0, palette.menu_stroke);
    v.menu_rounding = Rounding::same(8.0);

    v.selection.bg_fill = palette.selection;
    v.selection.stroke = Stroke::new(1.0, palette.accent);

    v.widgets.noninteractive.bg_fill = palette.window;
    v.widgets.noninteractive.weak_bg_fill = palette.window;
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.stroke);

    v.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, palette.text);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::TRANSPARENT);
    v.widgets.inactive.rounding = Rounding::same(4.0);

    v.widgets.hovered.bg_fill = palette.hover;
    v.widgets.hovered.weak_bg_fill = palette.hover;
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, palette.text);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::TRANSPARENT);
    v.widgets.hovered.rounding = Rounding::same(4.0);

    v.widgets.active.bg_fill = palette.pressed;
    v.widgets.active.weak_bg_fill = palette.pressed;
    v.widgets.active.fg_stroke = Stroke::new(1.0, palette.text);
    v.widgets.active.bg_stroke = Stroke::new(1.0, Color32::TRANSPARENT);
    v.widgets.active.rounding = Rounding::same(4.0);

    v.widgets.open.bg_fill = palette.hover;
    v.widgets.open.weak_bg_fill = palette.hover;
    v.widgets.open.fg_stroke = Stroke::new(1.0, palette.text);
    v.widgets.open.bg_stroke = Stroke::new(1.0, Color32::TRANSPARENT);

    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 4.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.interact_size = egui::vec2(28.0, 26.0);
    style.spacing.combo_width = 120.0;
    style.spacing.scroll.bar_width = 10.0;
    style.spacing.scroll.floating = true;

    style
}
