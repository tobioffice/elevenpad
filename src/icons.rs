//! Icons drawn with the painter so the app needs no icon font.

use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, Vec2};

pub fn plus(ui: &mut Ui, center: Pos2, color: Color32, size: f32) {
    let p = ui.painter();
    let s = size * 0.5;
    p.line_segment(
        [center + Vec2::new(-s, 0.0), center + Vec2::new(s, 0.0)],
        Stroke::new(1.2, color),
    );
    p.line_segment(
        [center + Vec2::new(0.0, -s), center + Vec2::new(0.0, s)],
        Stroke::new(1.2, color),
    );
}

pub fn cross(ui: &mut Ui, center: Pos2, color: Color32, size: f32) {
    let p = ui.painter();
    let s = size * 0.5;
    p.line_segment(
        [
            center + Vec2::new(-s, -s),
            center + Vec2::new(s, s),
        ],
        Stroke::new(1.2, color),
    );
    p.line_segment(
        [
            center + Vec2::new(-s, s),
            center + Vec2::new(s, -s),
        ],
        Stroke::new(1.2, color),
    );
}

pub fn chevron_down(ui: &mut Ui, center: Pos2, color: Color32, size: f32) {
    let p = ui.painter();
    let s = size * 0.5;
    p.line_segment(
        [center + Vec2::new(-s, -s * 0.4), center + Vec2::new(0.0, s * 0.5)],
        Stroke::new(1.2, color),
    );
    p.line_segment(
        [center + Vec2::new(0.0, s * 0.5), center + Vec2::new(s, -s * 0.4)],
        Stroke::new(1.2, color),
    );
}

pub fn chevron_up(ui: &mut Ui, center: Pos2, color: Color32, size: f32) {
    let p = ui.painter();
    let s = size * 0.5;
    p.line_segment(
        [center + Vec2::new(-s, s * 0.4), center + Vec2::new(0.0, -s * 0.5)],
        Stroke::new(1.2, color),
    );
    p.line_segment(
        [center + Vec2::new(0.0, -s * 0.5), center + Vec2::new(s, s * 0.4)],
        Stroke::new(1.2, color),
    );
}

pub fn search(ui: &mut Ui, center: Pos2, color: Color32, size: f32) {
    let p = ui.painter();
    let r = size * 0.32;
    let c = center + Vec2::new(-size * 0.08, -size * 0.08);
    p.circle_stroke(c, r, Stroke::new(1.2, color));
    p.line_segment(
        [
            c + Vec2::new(r * 0.72, r * 0.72),
            center + Vec2::new(size * 0.42, size * 0.42),
        ],
        Stroke::new(1.2, color),
    );
}

pub fn minimize(ui: &mut Ui, center: Pos2, color: Color32) {
    ui.painter().line_segment(
        [center + Vec2::new(-5.0, 0.0), center + Vec2::new(5.0, 0.0)],
        Stroke::new(1.0, color),
    );
}

pub fn maximize(ui: &mut Ui, center: Pos2, color: Color32) {
    let r = Rect::from_center_size(center, Vec2::splat(10.0));
    ui.painter()
        .rect_stroke(r, 0.0, Stroke::new(1.0, color));
}

pub fn restore(ui: &mut Ui, center: Pos2, color: Color32) {
    let p = ui.painter();
    let back = Rect::from_center_size(center + Vec2::new(1.0, -1.0), Vec2::splat(8.0));
    p.rect_stroke(back, 0.0, Stroke::new(1.0, color));
    let front = Rect::from_center_size(center + Vec2::new(-1.0, 1.0), Vec2::splat(8.0));
    p.rect_filled(front, 0.0, ui.style().visuals.window_fill);
    p.rect_stroke(front, 0.0, Stroke::new(1.0, color));
}

/// A 16x16 notebook mark for the tab strip.
pub fn notebook(ui: &Ui, rect: Rect, accent: Color32, text: Color32) {
    let p = ui.painter();
    let body = Rect::from_center_size(rect.center(), Vec2::new(13.0, 15.0));
    p.rect_filled(body, 2.0, accent);
    let page = body.shrink2(Vec2::new(2.5, 1.5));
    p.rect_filled(page, 1.0, Color32::WHITE);
    for i in 0..3 {
        let y = page.top() + 3.0 + i as f32 * 3.0;
        p.line_segment(
            [
                egui::pos2(page.left() + 1.5, y),
                egui::pos2(page.right() - 1.5, y),
            ],
            Stroke::new(0.8, text.gamma_multiply(0.55)),
        );
    }
    // spiral binding
    for i in 0..4 {
        let y = body.top() + 2.5 + i as f32 * 3.5;
        p.circle_filled(egui::pos2(body.left() + 1.0, y), 0.9, Color32::WHITE);
    }
}

pub fn clickable(ui: &Ui, rect: Rect, id: egui::Id) -> egui::Response {
    ui.interact(rect, id, Sense::click())
}


/// Four-point sparkle, used for the Copilot button.
pub fn sparkle(ui: &mut Ui, center: Pos2, color: Color32, radius: f32) {
    let mut pts = Vec::with_capacity(8);
    for i in 0..8 {
        let a = i as f32 * std::f32::consts::TAU / 8.0 - std::f32::consts::FRAC_PI_2;
        let r = if i % 2 == 0 { radius } else { radius * 0.30 };
        pts.push(center + Vec2::new(a.cos(), a.sin()) * r);
    }
    ui.painter().add(egui::Shape::convex_polygon(
        pts,
        color,
        Stroke::NONE,
    ));
}

/// Explorer-style folder icon.
pub fn folder(ui: &mut Ui, rect: Rect, tint: Color32) {
    let p = ui.painter();
    let w = rect.width();
    let h = rect.height();
    let tab = Rect::from_min_size(
        egui::pos2(rect.left() + w * 0.06, rect.top() + h * 0.18),
        Vec2::new(w * 0.42, h * 0.22),
    );
    p.rect_filled(tab, 2.0, tint.gamma_multiply(0.92));
    let body = Rect::from_min_size(
        egui::pos2(rect.left() + w * 0.04, rect.top() + h * 0.32),
        Vec2::new(w * 0.92, h * 0.56),
    );
    p.rect_filled(body, 2.5, tint);
    let lip = Rect::from_min_size(
        egui::pos2(body.left(), body.top()),
        Vec2::new(body.width(), body.height() * 0.22),
    );
    p.rect_filled(lip, 2.0, tint.gamma_multiply(1.12));
}

/// Document icon with a folded corner.
pub fn file_page(ui: &mut Ui, rect: Rect, page: Color32, ink: Color32) {
    let p = ui.painter();
    let w = rect.width();
    let h = rect.height();
    let body = Rect::from_min_size(
        egui::pos2(rect.left() + w * 0.18, rect.top() + h * 0.08),
        Vec2::new(w * 0.64, h * 0.84),
    );
    p.rect_filled(body, 1.5, page);
    p.rect_stroke(body, 1.5, Stroke::new(1.0, ink.gamma_multiply(0.75)));
    let fold = 4.0;
    p.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(body.right() - fold, body.top()),
            egui::pos2(body.right(), body.top() + fold),
            egui::pos2(body.right() - fold, body.top() + fold),
        ],
        ink.gamma_multiply(0.25),
        Stroke::NONE,
    ));
    for i in 0..3 {
        let y = body.top() + h * 0.34 + i as f32 * h * 0.16;
        p.line_segment(
            [
                egui::pos2(body.left() + 2.0, y),
                egui::pos2(body.right() - 2.0, y),
            ],
            Stroke::new(0.8, ink.gamma_multiply(0.75)),
        );
    }
}
