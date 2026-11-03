//! Windows 11 Notepad, reimplemented on Linux with egui.

use std::path::PathBuf;

use egui::{
    Align, Align2, Color32, FontId, Frame, Id, Key, Layout, Margin, Modifiers, Pos2, Rect,
    RichText, Rounding, Sense, Stroke, Vec2, ViewportCommand,
};

use crate::doc::{Doc, Eol};
use crate::icons;
use crate::syntax;
use crate::theme::{self, Palette, UI_FONT_SEMIBOLD};

pub struct FindState {
    pub query: String,
    pub replace: String,
    pub match_case: bool,
    pub wrap: bool,
    pub replace_mode: bool,
    pub focus: bool,
    pub message: Option<String>,
}

impl Default for FindState {
    fn default() -> Self {
        Self {
            query: String::new(),
            replace: String::new(),
            match_case: false,
            wrap: true,
            replace_mode: false,
            focus: false,
            message: None,
        }
    }
}

pub enum Dialog {
    Open {
        dir: PathBuf,
        entries: Vec<PathBuf>,
        selected: Option<PathBuf>,
        show_hidden: bool,
    },
    Save {
        dir: PathBuf,
        entries: Vec<PathBuf>,
        name: String,
        focus: bool,
        show_hidden: bool,
    },
    GoTo {
        value: String,
        focus: bool,
    },
    Font,
    About,
}

#[derive(Debug, Clone)]
pub enum Action {
    NewTab,
    CloseTab(usize),
    SelectTab(usize),
    OpenDialog,
    Save,
    SaveAs,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Delete,
    Find,
    Replace,
    FindNext(bool),
    GoTo,
    SelectAll,
    TimeDate,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ToggleWrap,
    ToggleDark,
    ToggleStatus,
    ToggleTop,
    ToggleRounded,
    Font,
    About,
    Minimize,
    Maximize,
    CloseWindow,
    CloseFind,
    ToggleReplace,
}

/// Rounded corners need a compositor that honors ARGB surfaces. Wayland does,
/// plain X11 without a compositing manager does not and shows black corners.
pub fn default_rounded() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
}

pub struct NotepadApp {
    pub docs: Vec<Doc>,
    pub active: usize,
    pub dark: bool,
    pub palette: Palette,
    pub word_wrap: bool,
    pub show_status: bool,
    pub zoom: f32,
    pub font_size: f32,
    pub find: FindState,
    pub find_visible: bool,
    pub dialog: Option<Dialog>,
    pub always_on_top: bool,
    pub rounded: bool,
    pub status_msg: Option<String>,
    pub status_time: f64,
    pub cursor: (usize, usize),
    last_shape_size: Vec2,
    last_shape_at: f64,
    last_ctx: Option<egui::Context>,
}

impl NotepadApp {
    pub fn new(ctx: &egui::Context) -> Self {
        let dark = false;
        theme::install_fonts(ctx);
        let palette = if dark { Palette::dark() } else { Palette::light() };
        ctx.set_style(theme::style(palette, dark));
        Self {
            docs: vec![Doc::new()],
            active: 0,
            dark,
            palette,
            word_wrap: true,
            show_status: true,
            zoom: 1.0,
            font_size: 15.0,
            find: FindState::default(),
            find_visible: false,
            dialog: None,
            always_on_top: false,
            rounded: default_rounded(),
            status_msg: None,
            status_time: 0.0,
            cursor: (1, 1),
            last_shape_size: Vec2::ZERO,
            last_shape_at: 0.0,
            last_ctx: Some(ctx.clone()),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        self.last_ctx = Some(ctx.clone());
        self.update_window_shape(ctx);
        let resize_zone = self.native_resize(ctx);
        let mut actions = Vec::new();
        self.handle_shortcuts(ctx, &mut actions);
        self.title_bar(ctx, &mut actions);
        self.menu_bar(ctx, &mut actions);
        if self.show_status {
            self.status_bar(ctx);
        }
        self.editor(ctx);
        self.find_bar(ctx, &mut actions);
        self.apply(actions, ctx);
        self.dialog_overlay(ctx);
        self.toast(ctx);
        if let Some(direction) = resize_zone {
            ctx.set_cursor_icon(resize_cursor(direction));
        }
    }

    /// Keep the X11 shape mask in sync with the window size when rounded corners
    /// are enabled. On Wayland this is a no-op and alpha does the work.
    fn update_window_shape(&mut self, ctx: &egui::Context) {
        if !self.rounded {
            return;
        }
        let size = ctx.screen_rect().size();
        let changed = (size - self.last_shape_size).length() > 1.0;
        if changed && now() - self.last_shape_at > 0.15 {
            self.last_shape_size = size;
            self.last_shape_at = now();
            let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            crate::shape::apply(size.x, size.y, if maximized { 0.0 } else { 12.0 });
        }
    }

    /// Edge hit testing for native window resizing. Returns the hovered edge.
    fn native_resize(&self, ctx: &egui::Context) -> Option<egui::viewport::ResizeDirection> {
        use egui::viewport::ResizeDirection as Dir;
        if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
            return None;
        }
        let pos = ctx.input(|i| i.pointer.hover_pos())?;
        let rect = ctx.screen_rect();
        let t = 6.0;
        let left = pos.x <= rect.left() + t;
        let right = pos.x >= rect.right() - t;
        let top = pos.y <= rect.top() + t;
        let bottom = pos.y >= rect.bottom() - t;
        if !(left || right || top || bottom) {
            return None;
        }
        let direction = match (left, right, top, bottom) {
            (true, _, true, _) => Dir::NorthWest,
            (_, true, true, _) => Dir::NorthEast,
            (true, _, _, true) => Dir::SouthWest,
            (_, true, _, true) => Dir::SouthEast,
            (true, _, _, _) => Dir::West,
            (_, true, _, _) => Dir::East,
            (_, _, true, _) => Dir::North,
            (_, _, _, true) => Dir::South,
            _ => return None,
        };
        if ctx.input(|i| i.pointer.primary_pressed()) {
            ctx.send_viewport_cmd(ViewportCommand::BeginResize(direction));
            // Keep the press away from the widgets underneath the border.
            ctx.input_mut(|i| {
                i.events.retain(|event| {
                    !matches!(
                        event,
                        egui::Event::PointerButton {
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            ..
                        }
                    )
                })
            });
        }
        Some(direction)
    }

    // ------------------------------------------------------------------ apply

    fn apply(&mut self, actions: Vec<Action>, ctx: &egui::Context) {
        for action in actions {
            match action {
                Action::NewTab => self.new_tab(),
                Action::CloseTab(i) => self.close_tab(i),
                Action::SelectTab(i) => self.active = i.min(self.docs.len().saturating_sub(1)),
                Action::OpenDialog => self.open_dialog(),
                Action::Save => self.save(false),
                Action::SaveAs => self.save(true),
                Action::Exit | Action::CloseWindow => {
                    ctx.send_viewport_cmd(ViewportCommand::Close)
                }
                Action::Undo => {
                    self.doc().undo();
                }
                Action::Redo => {
                    self.doc().redo();
                }
                Action::Cut => ctx.send_viewport_cmd(ViewportCommand::RequestCut),
                Action::Copy => ctx.send_viewport_cmd(ViewportCommand::RequestCopy),
                Action::Paste => ctx.send_viewport_cmd(ViewportCommand::RequestPaste),
                Action::Delete => self.delete_selection(ctx),
                Action::Find => self.open_find(false),
                Action::Replace => self.open_find(true),
                Action::FindNext(forward) => self.find_next(ctx, forward),
                Action::GoTo => {
                    self.dialog = Some(Dialog::GoTo {
                        value: String::new(),
                        focus: true,
                    })
                }
                Action::SelectAll => self.select_all(ctx),
                Action::TimeDate => {
                    let stamp = chrono::Local::now().format("%H:%M %d-%m-%Y").to_string();
                    self.insert_text(&stamp);
                }
                Action::ZoomIn => self.zoom = (self.zoom * 1.1).min(5.0),
                Action::ZoomOut => self.zoom = (self.zoom / 1.1).max(0.2),
                Action::ZoomReset => self.zoom = 1.0,
                Action::ToggleWrap => self.word_wrap = !self.word_wrap,
                Action::ToggleDark => {
                    self.dark = !self.dark;
                    self.palette = if self.dark { Palette::dark() } else { Palette::light() };
                    ctx.set_style(theme::style(self.palette, self.dark));
                }
                Action::ToggleStatus => self.show_status = !self.show_status,
                Action::ToggleTop => {
                    self.always_on_top = !self.always_on_top;
                    ctx.send_viewport_cmd(ViewportCommand::WindowLevel(if self.always_on_top {
                        egui::viewport::WindowLevel::AlwaysOnTop
                    } else {
                        egui::viewport::WindowLevel::Normal
                    }));
                }
                Action::ToggleRounded => {
                    self.rounded = !self.rounded;
                    ctx.send_viewport_cmd(ViewportCommand::Transparent(self.rounded));
                    let size = ctx.screen_rect().size();
                    crate::shape::apply(size.x, size.y, if self.rounded { 12.0 } else { 0.0 });
                }
                Action::Font => self.dialog = Some(Dialog::Font),
                Action::About => self.dialog = Some(Dialog::About),
                Action::CloseFind => self.find_visible = false,
                Action::ToggleReplace => {
                    self.find.replace_mode = !self.find.replace_mode;
                    self.find.focus = true;
                }
                Action::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
                Action::Maximize => {
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
                }
            }
        }
    }

    fn flash(&mut self, msg: impl Into<String>) {
        self.status_msg = Some(msg.into());
        self.status_time = now();
    }

    fn new_tab(&mut self) {
        self.docs.push(Doc::new());
        self.active = self.docs.len() - 1;
    }

    fn close_tab(&mut self, index: usize) {
        if index >= self.docs.len() {
            return;
        }
        if self.docs[index].modified {
            self.flash("Unsaved changes discarded");
        }
        self.docs.remove(index);
        if self.docs.is_empty() {
            self.docs.push(Doc::new());
            self.active = 0;
        } else if self.active >= self.docs.len() {
            self.active = self.docs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
    }

    fn doc_index(&self) -> usize {
        self.active.min(self.docs.len().saturating_sub(1))
    }

    fn doc(&mut self) -> &mut Doc {
        let index = self.active.min(self.docs.len() - 1);
        &mut self.docs[index]
    }

    // -------------------------------------------------------------- title bar

    fn title_bar(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let p = self.palette;
        egui::TopBottomPanel::top("title-bar")
            .exact_height(40.0)
            .frame(Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let pointer = ui.input(|i| i.pointer.hover_pos());
                let radius = if self.rounded { 12.0 } else { 0.0 };
                let body = Rect::from_min_max(rect.min, Pos2::new(rect.right(), rect.bottom() + 1.0));
                let rounding = Rounding {
                    nw: radius,
                    ne: radius,
                    sw: 0.0,
                    se: 0.0,
                };
                ui.painter().rect_filled(body, rounding, p.window);
                if self.rounded {
                    ui.painter().rect_stroke(
                        body.shrink(0.5),
                        rounding,
                        Stroke::new(1.0, p.stroke_strong),
                    );
                }

                let caption_w = 138.0;
                let mut x = rect.left() + 8.0;
                let avail = rect.width() - caption_w - 16.0;
                let count = self.docs.len().max(1) as f32;
                let tab_w = ((avail - 36.0) / count).clamp(120.0, 220.0);

                for i in 0..self.docs.len() {
                    let tab_rect =
                        Rect::from_min_size(Pos2::new(x, rect.top() + 8.0), Vec2::new(tab_w, 32.0));
                    let resp = ui.interact(tab_rect, Id::new(("tab", i)), Sense::click());
                    let active = i == self.active;
                    let close_rect = Rect::from_center_size(
                        Pos2::new(tab_rect.right() - 16.0, tab_rect.center().y),
                        Vec2::splat(18.0),
                    );
                    let close_resp =
                        ui.interact(close_rect, Id::new(("tab-close", i)), Sense::click());
                    // The close button sits on top of the tab and steals `hovered()`
                    // from it, so use the pointer position for the shared hover state.
                    let over_tab = pointer.map(|p| tab_rect.contains(p)).unwrap_or(false);
                    let over_close = pointer.map(|p| close_rect.contains(p)).unwrap_or(false);
                    let hovered = resp.hovered() || over_tab || over_close;

                    let painter = ui.painter();
                    if active {
                        painter.rect_filled(
                            Rect::from_min_max(
                                tab_rect.min,
                                Pos2::new(tab_rect.right(), rect.bottom() + 1.0),
                            ),
                            Rounding {
                                nw: 8.0,
                                ne: 8.0,
                                sw: 0.0,
                                se: 0.0,
                            },
                            p.surface,
                        );
                    } else if hovered {
                        painter.rect_filled(tab_rect, Rounding::same(6.0), p.tab_hover);
                    }

                    let title = self.docs[i].title();
                    let modified = self.docs[i].modified;
                    let text_color = if active { p.text } else { p.text_weak };
                    let text_left = tab_rect.left() + 30.0;
                    let text_right = tab_rect.right() - 30.0;
                    let icon_rect = Rect::from_center_size(
                        Pos2::new(tab_rect.left() + 16.0, tab_rect.center().y),
                        Vec2::splat(14.0),
                    );
                    icons::notebook(ui, icon_rect, p.accent, p.text);
                    painter
                        .with_clip_rect(Rect::from_min_max(
                            Pos2::new(text_left, tab_rect.top()),
                            Pos2::new(text_right, tab_rect.bottom()),
                        ))
                        .text(
                            Pos2::new(text_left, tab_rect.center().y),
                            Align2::LEFT_CENTER,
                            &title,
                            FontId::new(12.5, egui::FontFamily::Proportional),
                            text_color,
                        );

                    if modified && !hovered {
                        painter.circle_filled(
                            Pos2::new(tab_rect.right() - 16.0, tab_rect.center().y),
                            3.0,
                            p.text_weak,
                        );
                    } else if hovered || active {
                        if over_close {
                            painter.rect_filled(close_rect, Rounding::same(4.0), p.pressed);
                        }
                        icons::cross(
                            ui,
                            close_rect.center(),
                            if over_close { p.text } else { p.text_weak },
                            9.0,
                        );
                    }

                    if resp.clicked() {
                        actions.push(Action::SelectTab(i));
                    }
                    if close_resp.clicked() {
                        actions.push(Action::CloseTab(i));
                    }
                    x += tab_w + 2.0;
                }

                let plus_rect =
                    Rect::from_center_size(Pos2::new(x + 16.0, rect.top() + 24.0), Vec2::splat(28.0));
                let plus = ui.interact(plus_rect, Id::new("new-tab"), Sense::click());
                if plus.hovered() {
                    ui.painter().rect_filled(plus_rect, Rounding::same(4.0), p.hover);
                }
                icons::plus(ui, plus_rect.center(), p.text, 9.5);
                if plus.clicked() {
                    actions.push(Action::NewTab);
                }

                // Empty strip to the right of the tabs moves the window. The command is
                // sent on the press itself, not after a drag threshold, because Wayland
                // move requests are tied to the serial of the press that started them.
                let drag_left = plus_rect.right() + 4.0;
                let drag_right = rect.right() - caption_w;
                if drag_right > drag_left {
                    let drag_rect = Rect::from_min_max(
                        Pos2::new(drag_left, rect.top()),
                        Pos2::new(drag_right, rect.bottom()),
                    );
                    let drag = ui.interact(drag_rect, Id::new("title-drag"), Sense::click_and_drag());
                    let pressed = drag.contains_pointer() && ctx.input(|i| i.pointer.primary_pressed());
                    if pressed || drag.drag_started() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                    if drag.double_clicked() {
                        actions.push(Action::Maximize);
                    }
                }

                let mut bx = rect.right() - caption_w;
                for (id, kind) in [
                    (Id::new("cap-min"), 0u8),
                    (Id::new("cap-max"), 1u8),
                    (Id::new("cap-close"), 2u8),
                ] {
                    let brect = Rect::from_min_size(Pos2::new(bx, rect.top()), Vec2::new(46.0, 40.0));
                    let resp = ui.interact(brect, id, Sense::click());
                    let hovered = pointer.map(|p| brect.contains(p)).unwrap_or(false);
                    let close = kind == 2;
                    if hovered {
                        ui.painter().rect_filled(
                            brect,
                            0.0,
                            if close { p.close_hover } else { p.hover },
                        );
                    }
                    let color = if close && hovered { Color32::WHITE } else { p.text };
                    let center = brect.center();
                    match kind {
                        0 => icons::minimize(ui, center, color),
                        1 => {
                            if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
                                icons::restore(ui, center, color)
                            } else {
                                icons::maximize(ui, center, color)
                            }
                        }
                        _ => icons::cross(ui, center, color, 10.0),
                    }
                    if resp.clicked() {
                        actions.push(match kind {
                            0 => Action::Minimize,
                            1 => Action::Maximize,
                            _ => Action::CloseWindow,
                        });
                    }
                    bx += 46.0;
                }
            });
    }

    // --------------------------------------------------------------- menu bar

    fn menu_bar(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        let p = self.palette;
        let word_wrap = self.word_wrap;
        let dark = self.dark;
        let status = self.show_status;
        let top = self.always_on_top;
        let rounded = self.rounded;

        egui::TopBottomPanel::top("menu-bar")
            .exact_height(38.0)
            .frame(Frame::none().fill(p.window))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(6.0);
                    ui.spacing_mut().button_padding = Vec2::new(10.0, 5.0);
                    let label = |text: &str| RichText::new(text).size(12.5).color(p.text);

                    ui.menu_button(label("File"), |ui| {
                        ui.set_min_width(250.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        if item(ui, "New tab", "Ctrl+T", true, None, p) {
                            actions.push(Action::NewTab);
                            ui.close_menu();
                        }
                        if item(ui, "New window", "Ctrl+Shift+N", false, None, p) {
                            ui.close_menu();
                        }
                        separator(ui, p);
                        if item(ui, "Open...", "Ctrl+O", true, None, p) {
                            actions.push(Action::OpenDialog);
                            ui.close_menu();
                        }
                        if item(ui, "Save", "Ctrl+S", true, None, p) {
                            actions.push(Action::Save);
                            ui.close_menu();
                        }
                        if item(ui, "Save as...", "Ctrl+Shift+S", true, None, p) {
                            actions.push(Action::SaveAs);
                            ui.close_menu();
                        }
                        separator(ui, p);
                        let _ = item(ui, "Page setup...", "", false, None, p);
                        let _ = item(ui, "Print...", "Ctrl+P", false, None, p);
                        separator(ui, p);
                        if item(ui, "Exit", "", true, None, p) {
                            actions.push(Action::Exit);
                            ui.close_menu();
                        }
                    });

                    ui.menu_button(label("Edit"), |ui| {
                        ui.set_min_width(250.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        if item(ui, "Undo", "Ctrl+Z", true, None, p) {
                            actions.push(Action::Undo);
                            ui.close_menu();
                        }
                        if item(ui, "Redo", "Ctrl+Y", true, None, p) {
                            actions.push(Action::Redo);
                            ui.close_menu();
                        }
                        separator(ui, p);
                        if item(ui, "Cut", "Ctrl+X", true, None, p) {
                            actions.push(Action::Cut);
                            ui.close_menu();
                        }
                        if item(ui, "Copy", "Ctrl+C", true, None, p) {
                            actions.push(Action::Copy);
                            ui.close_menu();
                        }
                        if item(ui, "Paste", "Ctrl+V", true, None, p) {
                            actions.push(Action::Paste);
                            ui.close_menu();
                        }
                        if item(ui, "Delete", "Del", true, None, p) {
                            actions.push(Action::Delete);
                            ui.close_menu();
                        }
                        separator(ui, p);
                        if item(ui, "Find", "Ctrl+F", true, None, p) {
                            actions.push(Action::Find);
                            ui.close_menu();
                        }
                        if item(ui, "Find next", "F3", true, None, p) {
                            actions.push(Action::FindNext(true));
                            ui.close_menu();
                        }
                        if item(ui, "Find previous", "Shift+F3", true, None, p) {
                            actions.push(Action::FindNext(false));
                            ui.close_menu();
                        }
                        if item(ui, "Replace", "Ctrl+H", true, None, p) {
                            actions.push(Action::Replace);
                            ui.close_menu();
                        }
                        if item(ui, "Go to...", "Ctrl+G", true, None, p) {
                            actions.push(Action::GoTo);
                            ui.close_menu();
                        }
                        separator(ui, p);
                        if item(ui, "Select all", "Ctrl+A", true, None, p) {
                            actions.push(Action::SelectAll);
                            ui.close_menu();
                        }
                        if item(ui, "Time/Date", "F5", true, None, p) {
                            actions.push(Action::TimeDate);
                            ui.close_menu();
                        }
                    });

                    ui.menu_button(label("View"), |ui| {
                        ui.set_min_width(250.0);
                        ui.spacing_mut().item_spacing.y = 1.0;
                        if item(ui, "Zoom in", "Ctrl++", true, None, p) {
                            actions.push(Action::ZoomIn);
                            ui.close_menu();
                        }
                        if item(ui, "Zoom out", "Ctrl+-", true, None, p) {
                            actions.push(Action::ZoomOut);
                            ui.close_menu();
                        }
                        if item(ui, "Restore default zoom", "Ctrl+0", true, None, p) {
                            actions.push(Action::ZoomReset);
                            ui.close_menu();
                        }
                        separator(ui, p);
                        if item(ui, "Word wrap", "", true, Some(word_wrap), p) {
                            actions.push(Action::ToggleWrap);
                        }
                        separator(ui, p);
                        if item(ui, "Font...", "", true, None, p) {
                            actions.push(Action::Font);
                            ui.close_menu();
                        }
                        if item(ui, "Dark mode", "", true, Some(dark), p) {
                            actions.push(Action::ToggleDark);
                        }
                        if item(ui, "Status bar", "", true, Some(status), p) {
                            actions.push(Action::ToggleStatus);
                        }
                        if item(ui, "Always on top", "", true, Some(top), p) {
                            actions.push(Action::ToggleTop);
                        }
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(8.0);
                        avatar(ui, p);
                        if settings_button(ui, p).clicked() {
                            actions.push(Action::About);
                        }
                        if cowriter_button(ui, p).clicked() {
                            actions.push(Action::About);
                        }
                        if formatting_button(ui, p).clicked() {
                            actions.push(Action::Font);
                        }
                    });
                });
                if self.rounded {
                    let rect = ui.max_rect();
                    let stroke = Stroke::new(1.0, p.stroke_strong);
                    ui.painter().line_segment(
                        [Pos2::new(rect.left() + 1.0, rect.top()), Pos2::new(rect.left() + 1.0, rect.bottom())],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [Pos2::new(rect.right() - 1.0, rect.top()), Pos2::new(rect.right() - 1.0, rect.bottom())],
                        stroke,
                    );
                }
            });
    }

    // ----------------------------------------------------------------- editor

    fn editor(&mut self, ctx: &egui::Context) {
        let p = self.palette;
        let editor_id = Id::new("notepad-editor");
        let font_size = self.font_size * self.zoom;
        let wrap = self.word_wrap;
        let mut cursor = self.cursor;

        egui::CentralPanel::default()
            .frame(Frame::none().fill(p.surface))
            .show(ctx, |ui| {
                let index = self.active.min(self.docs.len() - 1);
                let doc = &mut self.docs[index];
                let before = doc.text.clone();

                let font_id = FontId::monospace(font_size);
                let lang = syntax::detect(doc.path.as_deref());
                let syn_colors = if self.dark {
                    syntax::Colors::dark()
                } else {
                    syntax::Colors::light()
                };
                let layout_font = font_id.clone();
                let base_color = p.text;
                let mut layouter = move |ui: &egui::Ui, text: &str, wrap_width: f32| {
                    let job = syntax::highlight(
                        text,
                        lang,
                        layout_font.clone(),
                        base_color,
                        syn_colors,
                        wrap_width,
                    );
                    ui.fonts(|f| f.layout_job(job))
                };
                let row_height = ui.fonts(|f| f.row_height(&font_id));
                let rows = ((ui.available_height().max(0.0) / row_height).ceil() as usize).max(1);

                let width = if wrap {
                    (ui.available_width() - 20.0).max(100.0)
                } else {
                    let galley = ui.fonts(|f| {
                        f.layout(doc.text.clone(), font_id.clone(), p.text, f32::INFINITY)
                    });
                    (galley.size().x + 28.0).max(ui.available_width())
                };

                let mut scroll = egui::ScrollArea::both().auto_shrink([false, false]);
                if let Some(line) = doc.scroll_to_line.take() {
                    let offset = (line.saturating_sub(1) as f32) * font_size * 1.35;
                    scroll = scroll.vertical_scroll_offset(offset);
                }

                let output = scroll.show(ui, |ui| {
                    egui::TextEdit::multiline(&mut doc.text)
                        .id(editor_id)
                        .font(font_id.clone())
                        .desired_rows(rows)
                        .text_color(p.text)
                        .frame(false)
                        .margin(Margin {
                            left: 12.0,
                            right: 12.0,
                            top: 8.0,
                            bottom: 24.0,
                        })
                        .desired_width(width)
                        .layouter(&mut layouter)
                        .lock_focus(true)
                        .show(ui)
                });

                if let Some(range) = output.inner.cursor_range {
                    cursor = doc.line_col(range.primary.ccursor.index);
                }
                if doc.text != before {
                    doc.snapshot(before);
                    doc.modified = true;
                }
                if output.inner.response.clicked() {
                    output.inner.response.request_focus();
                }
                if self.rounded {
                    let full = ui.max_rect();
                    let stroke = Stroke::new(1.0, p.stroke_strong);
                    ui.painter().line_segment(
                        [Pos2::new(full.left() + 1.0, full.top()), Pos2::new(full.left() + 1.0, full.bottom())],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [Pos2::new(full.right() - 1.0, full.top()), Pos2::new(full.right() - 1.0, full.bottom())],
                        stroke,
                    );
                }
            });



        self.cursor = cursor;
    }

    // ------------------------------------------------------------- status bar

    fn status_bar(&mut self, ctx: &egui::Context) {
        let p = self.palette;
        let (line, col) = self.cursor;
        let zoom = (self.zoom * 100.0).round() as i32;
        let eol = self.docs[self.doc_index()].eol;

        let characters = self.docs[self.doc_index()].text.chars().count();
        egui::TopBottomPanel::bottom("status-bar")
            .exact_height(28.0)
            .frame(Frame::none().fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let radius = if self.rounded { 12.0 } else { 0.0 };
                let rounding = Rounding {
                    nw: 0.0,
                    ne: 0.0,
                    sw: radius,
                    se: radius,
                };
                ui.painter().rect_filled(rect, rounding, p.window);
                ui.painter().line_segment(
                    [rect.left_top(), rect.right_top()],
                    Stroke::new(1.0, p.stroke),
                );
                if self.rounded {
                    ui.painter().rect_stroke(
                        rect.shrink(0.5),
                        rounding,
                        Stroke::new(1.0, p.stroke_strong),
                    );
                }
                let painter = ui.painter();
                let cursor_text = format!("Ln {line}, Col {col}");
                painter.text(
                    Pos2::new(rect.left() + 12.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    &cursor_text,
                    FontId::proportional(12.0),
                    p.text_weak,
                );
                let cursor_w = painter
                    .layout_no_wrap(cursor_text, FontId::proportional(12.0), p.text_weak)
                    .size()
                    .x;
                let sep_x = rect.left() + 12.0 + cursor_w + 10.0;
                painter.line_segment(
                    [
                        Pos2::new(sep_x, rect.center().y - 6.0),
                        Pos2::new(sep_x, rect.center().y + 6.0),
                    ],
                    Stroke::new(1.0, p.stroke_strong),
                );
                let chars_text = format!("{characters} characters");
                let chars_w = painter
                    .layout_no_wrap(chars_text.clone(), FontId::proportional(12.0), p.text_weak)
                    .size()
                    .x;
                painter.text(
                    Pos2::new(sep_x + 10.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    &chars_text,
                    FontId::proportional(12.0),
                    p.text_weak,
                );
                let plain_sep = sep_x + 10.0 + chars_w + 12.0;
                painter.line_segment(
                    [
                        Pos2::new(plain_sep, rect.center().y - 6.0),
                        Pos2::new(plain_sep, rect.center().y + 6.0),
                    ],
                    Stroke::new(1.0, p.stroke_strong),
                );
                painter.text(
                    Pos2::new(plain_sep + 10.0, rect.center().y),
                    Align2::LEFT_CENTER,
                    "Aa  Plain",
                    FontId::proportional(12.0),
                    p.text_weak,
                );

                let mut right = rect.right() - 14.0;
                let items = [
                    "UTF-8".to_string(),
                    eol.label().to_string(),
                    format!("{zoom}%"),
                ];
                for (i, text) in items.iter().enumerate() {
                    let galley = painter.layout_no_wrap(
                        text.clone(),
                        FontId::proportional(11.5),
                        p.text_weak,
                    );
                    let w = galley.size().x;
                    painter.galley(
                        Pos2::new(right - w, rect.center().y - galley.size().y * 0.5),
                        galley,
                        p.text_weak,
                    );
                    right -= w + 10.0;
                    if i < 2 {
                        painter.line_segment(
                            [
                                Pos2::new(right, rect.center().y - 6.0),
                                Pos2::new(right, rect.center().y + 6.0),
                            ],
                            Stroke::new(1.0, p.stroke_strong),
                        );
                        right -= 9.0;
                    }
                }
            });
    }

    // ------------------------------------------------------------------- find

    fn open_find(&mut self, replace: bool) {
        self.find_visible = true;
        self.find.replace_mode = replace;
        self.find.focus = true;
    }

    fn find_bar(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if !self.find_visible {
            return;
        }
        let p = self.palette;
        let replace_mode = self.find.replace_mode;
        let height = if replace_mode { 96.0 } else { 52.0 };

        egui::TopBottomPanel::top("find-bar")
            .exact_height(height)
            .frame(
                Frame::none()
                    .fill(p.surface_alt)
                    .stroke(Stroke::new(1.0, p.stroke)),
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new(if replace_mode { "Replace" } else { "Find" })
                            .size(12.0)
                            .color(p.text_weak),
                    );
                    ui.add_space(4.0);
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut self.find.query)
                            .desired_width(240.0)
                            .hint_text("Find"),
                    );
                    if self.find.focus {
                        resp.request_focus();
                        self.find.focus = false;
                    }
                    if replace_mode {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.find.replace)
                                .desired_width(240.0)
                                .hint_text("Replace with"),
                        );
                    }
                    ui.checkbox(&mut self.find.match_case, "Match case");
                    ui.checkbox(&mut self.find.wrap, "Wrap around");
                    if replace_mode {
                        if ui.button("Replace").clicked() {
                            self.replace_one();
                        }
                        if ui.button("Replace all").clicked() {
                            self.replace_all();
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(6.0);
                        if ui.button("✕").on_hover_text("Close").clicked() {
                            actions.push(Action::CloseFind);
                        }
                        if ui.button("↓").on_hover_text("Find previous").clicked() {
                            actions.push(Action::FindNext(false));
                        }
                        if ui.button("↑").on_hover_text("Find next").clicked() {
                            actions.push(Action::FindNext(true));
                        }
                    });
                });
                if replace_mode {
                    ui.horizontal(|ui| {
                        ui.add_space(60.0);
                        if let Some(msg) = &self.find.message {
                            ui.label(RichText::new(msg).size(11.0).color(p.text_weak));
                        }
                    });
                }
            });
    }

    fn find_range(&self) -> Option<(usize, usize)> {
        let idx = self.doc_index();
        let doc = &self.docs[idx];
        let needle = if self.find.match_case {
            self.find.query.clone()
        } else {
            self.find.query.to_lowercase()
        };
        if needle.is_empty() {
            return None;
        }
        let haystack = if self.find.match_case {
            doc.text.clone()
        } else {
            doc.text.to_lowercase()
        };
        let start_char = self.char_index();
        let start_byte = doc
            .text
            .char_indices()
            .nth(start_char)
            .map(|(b, _)| b)
            .unwrap_or(doc.text.len());
        let forward = haystack[start_byte.min(haystack.len())..].find(&needle);
        let byte_start = match forward {
            Some(offset) => start_byte + offset,
            None if self.find.wrap => haystack.find(&needle)?,
            None => return None,
        };
        let char_start = haystack[..byte_start].chars().count();
        Some((char_start, char_start + needle.chars().count()))
    }

    fn char_index(&self) -> usize {
        let Some(ctx) = self.last_ctx.clone() else {
            return 0;
        };
        let id = Id::new("notepad-editor");
        egui::text_edit::TextEditState::load(&ctx, id)
            .and_then(|state| state.cursor.char_range())
            .map(|range| range.primary.index)
            .unwrap_or(0)
    }

    fn find_next(&mut self, ctx: &egui::Context, forward: bool) {
        if self.find.query.is_empty() {
            self.find.message = Some("Enter text to find".into());
            self.open_find(self.find.replace_mode);
            return;
        }
        if forward {
            match self.find_range() {
                Some((start, end)) => {
                    self.select_chars(ctx, start, end);
                    self.find.message = None;
                }
                None => self.find.message = Some("No matches found".into()),
            }
        } else {
            self.find_backwards(ctx);
        }
    }

    fn find_backwards(&mut self, ctx: &egui::Context) {
        let idx = self.doc_index();
        let doc = &self.docs[idx];
        let needle = if self.find.match_case {
            self.find.query.clone()
        } else {
            self.find.query.to_lowercase()
        };
        let haystack = if self.find.match_case {
            doc.text.clone()
        } else {
            doc.text.to_lowercase()
        };
        if needle.is_empty() {
            return;
        }
        let start_char = self.char_index();
        let start_byte = doc
            .text
            .char_indices()
            .nth(start_char)
            .map(|(b, _)| b)
            .unwrap_or(doc.text.len());
        let found = haystack[..start_byte.min(haystack.len())].rfind(&needle);
        if let Some(byte_start) = found {
            let char_start = haystack[..byte_start].chars().count();
            self.select_chars(ctx, char_start, char_start + needle.chars().count());
            self.find.message = None;
        } else {
            self.find.message = Some("No matches found".into());
        }
    }

    fn select_chars(&self, ctx: &egui::Context, start: usize, end: usize) {
        let id = Id::new("notepad-editor");
        if let Some(mut state) = egui::text_edit::TextEditState::load(ctx, id) {
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(start),
                    egui::text::CCursor::new(end),
                )));
            state.store(ctx, id);
        }
    }

    fn replace_one(&mut self) {
        let Some((start, end)) = self.find_range() else {
            self.find.message = Some("No matches found".into());
            return;
        };
        let idx = self.doc_index();
        let doc = &mut self.docs[idx];
        let before = doc.text.clone();
        let mut chars: Vec<char> = doc.text.chars().collect();
        let replacement: Vec<char> = self.find.replace.chars().collect();
        chars.splice(start..end, replacement);
        doc.text = chars.into_iter().collect();
        doc.snapshot(before);
        doc.modified = true;
    }

    fn replace_all(&mut self) {
        let idx = self.doc_index();
        let doc = &mut self.docs[idx];
        let before = doc.text.clone();
        let mut count = 0;
        let result = if self.find.match_case {
            count = doc.text.matches(&self.find.query).count();
            doc.text.replace(&self.find.query, &self.find.replace)
        } else {
            let mut result = String::new();
            let lower = doc.text.to_lowercase();
            let needle = self.find.query.to_lowercase();
            let mut last = 0;
            let mut search = 0;
            while let Some(pos) = lower[search..].find(&needle) {
                let start = search + pos;
                result.push_str(&doc.text[last..start]);
                result.push_str(&self.find.replace);
                count += 1;
                last = start + needle.len();
                search = last;
            }
            result.push_str(&doc.text[last..]);
            result
        };
        if count == 0 {
            self.find.message = Some("No matches found".into());
            return;
        }
        doc.text = result;
        doc.snapshot(before);
        doc.modified = true;
        self.find.message = Some(format!("Replaced {count} occurrence(s)"));
    }

    fn insert_text(&mut self, text: &str) {
        let Some(ctx) = self.last_ctx.clone() else {
            return;
        };
        let id = Id::new("notepad-editor");
        let pos = egui::text_edit::TextEditState::load(&ctx, id)
            .and_then(|s| s.cursor.char_range())
            .map(|r| r.primary.index)
            .unwrap_or(0);
        let idx = self.doc_index();
        let doc = &mut self.docs[idx];
        let before = doc.text.clone();
        let mut chars: Vec<char> = doc.text.chars().collect();
        let insert: Vec<char> = text.chars().collect();
        let n = insert.len();
        chars.splice(pos..pos, insert);
        doc.text = chars.into_iter().collect();
        doc.snapshot(before);
        doc.modified = true;
        self.select_chars(&ctx, pos + n, pos + n);
    }

    fn delete_selection(&mut self, ctx: &egui::Context) {
        let id = Id::new("notepad-editor");
        let Some(range) = egui::text_edit::TextEditState::load(ctx, id).and_then(|s| s.cursor.char_range())
        else {
            return;
        };
        if range.primary.index == range.secondary.index {
            return;
        }
        let (start, end) = (
            range.primary.index.min(range.secondary.index),
            range.primary.index.max(range.secondary.index),
        );
        let idx = self.doc_index();
        let doc = &mut self.docs[idx];
        let before = doc.text.clone();
        let mut chars: Vec<char> = doc.text.chars().collect();
        chars.drain(start..end);
        doc.text = chars.into_iter().collect();
        doc.snapshot(before);
        doc.modified = true;
        self.select_chars(ctx, start, start);
    }

    fn select_all(&mut self, ctx: &egui::Context) {
        let len = self.docs[self.doc_index()]
            .text
            .chars()
            .count();
        self.select_chars(ctx, 0, len);
    }

    // ---------------------------------------------------------------- dialogs

    fn start_dir(&self) -> PathBuf {
        self.docs[self.doc_index()]
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(|p| p.to_path_buf())
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("/"))
    }

    fn open_dialog(&mut self) {
        let dir = self.start_dir();
        let entries = list_dir(&dir, false);
        self.dialog = Some(Dialog::Open {
            dir,
            entries,
            selected: None,
            show_hidden: false,
        });
    }

    pub fn save(&mut self, force_dialog: bool) {
        let idx = self.doc_index();
        let doc = &mut self.docs[idx];
        if let (Some(path), false) = (doc.path.clone(), force_dialog) {
            match doc.save_to(path.clone()) {
                Ok(()) => self.flash(format!("Saved {}", path.display())),
                Err(err) => self.flash(format!("Save failed: {err}")),
            }
        } else {
            let dir = doc
                .path
                .as_ref()
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("/"));
            let entries = list_dir(&dir, false);
            let name = if doc.title() == "Untitled" {
                "Untitled.txt".into()
            } else {
                doc.title()
            };
            self.dialog = Some(Dialog::Save {
                dir,
                entries,
                name,
                focus: true,
                show_hidden: false,
            });
        }
    }

    fn dialog_overlay(&mut self, ctx: &egui::Context) {
        let Some(dialog) = self.dialog.take() else {
            return;
        };
        let p = self.palette;
        let mut close = false;
        let mut state = dialog;
        let mut open_path: Option<PathBuf> = None;
        let mut save_to: Option<PathBuf> = None;
        let mut goto_line: Option<usize> = None;

        egui::Area::new(Id::new("dialog-backdrop"))
            .fixed_pos(ctx.screen_rect().min)
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                let rect = ctx.screen_rect();
                ui.painter()
                    .rect_filled(rect, 0.0, Color32::from_black_alpha(70));
                let _ = ui.allocate_rect(rect, Sense::click());
            });

        egui::Area::new(Id::new("dialog"))
            .fixed_pos(ctx.screen_rect().center() - Vec2::new(270.0, 190.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                Frame::none()
                    .fill(p.menu_fill)
                    .stroke(Stroke::new(1.0, p.menu_stroke))
                    .rounding(Rounding::same(8.0))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 8.0),
                        blur: 24.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(60),
                    })
                    .inner_margin(Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.set_width(520.0);
                        match &mut state {
                            Dialog::Open {
                                dir,
                                entries,
                                selected,
                                show_hidden,
                            } => {
                                ui.label(RichText::new("Open").size(15.0).strong());
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(dir.display().to_string())
                                        .size(11.0)
                                        .color(p.text_weak),
                                );
                                ui.add_space(6.0);
                                if ui.button("Up one level").clicked() {
                                    if let Some(parent) = dir.parent() {
                                        *dir = parent.to_path_buf();
                                        *entries = list_dir(dir, *show_hidden);
                                        *selected = None;
                                    }
                                }
                                let mut navigate: Option<PathBuf> = None;
                                egui::ScrollArea::vertical()
                                    .max_height(230.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for entry in entries.iter() {
                                            let name = entry
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            let here = selected.as_ref() == Some(entry);
                                            if file_row(ui, &name, entry.is_dir(), here, p) {
                                                if entry.is_dir() {
                                                    navigate = Some(entry.clone());
                                                } else {
                                                    *selected = Some(entry.clone());
                                                }
                                            }
                                        }
                                    });
                                if let Some(target) = navigate.take() {
                                    *dir = target;
                                    *entries = list_dir(dir, *show_hidden);
                                    *selected = None;
                                }
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(show_hidden, "Show hidden items")
                                        .changed()
                                    {
                                        *entries = list_dir(dir, *show_hidden);
                                        *selected = None;
                                    }
                                    ui.add_space(12.0);
                                    if ui.button("Cancel").clicked() {
                                        close = true;
                                    }
                                    if ui.button("Open").clicked() {
                                        if let Some(path) = selected.clone() {
                                            if path.is_dir() {
                                                *dir = path;
                                                *entries = list_dir(dir, *show_hidden);
                                                *selected = None;
                                            } else {
                                                open_path = Some(path);
                                                close = true;
                                            }
                                        }
                                    }
                                });
                            }
                            Dialog::Save {
                                dir,
                                entries,
                                name,
                                focus,
                                show_hidden,
                            } => {
                                ui.label(RichText::new("Save as").size(15.0).strong());
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(dir.display().to_string())
                                        .size(11.0)
                                        .color(p.text_weak),
                                );
                                ui.add_space(6.0);
                                let mut navigate: Option<PathBuf> = None;
                                egui::ScrollArea::vertical()
                                    .max_height(190.0)
                                    .auto_shrink([false, false])
                                    .show(ui, |ui| {
                                        for entry in entries.iter() {
                                            let entry_name = entry
                                                .file_name()
                                                .map(|n| n.to_string_lossy().to_string())
                                                .unwrap_or_default();
                                            if file_row(ui, &entry_name, entry.is_dir(), false, p) {
                                                if entry.is_dir() {
                                                    navigate = Some(entry.clone());
                                                } else {
                                                    *name = entry_name;
                                                }
                                            }
                                        }
                                    });
                                if let Some(target) = navigate.take() {
                                    *dir = target;
                                    *entries = list_dir(dir, *show_hidden);
                                }
                                ui.add_space(6.0);
                                ui.horizontal(|ui| {
                                    ui.label("File name:");
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(name).desired_width(380.0),
                                    );
                                    if *focus {
                                        resp.request_focus();
                                        *focus = false;
                                    }
                                });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui
                                        .checkbox(show_hidden, "Show hidden items")
                                        .changed()
                                    {
                                        *entries = list_dir(dir, *show_hidden);
                                    }
                                    ui.add_space(12.0);
                                    if ui.button("Cancel").clicked() {
                                        close = true;
                                    }
                                    if ui.button("Save").clicked() {
                                        save_to = Some(dir.join(name.clone()));
                                        close = true;
                                    }
                                });
                            }
                            Dialog::GoTo { value, focus } => {
                                ui.label(RichText::new("Go to line").size(15.0).strong());
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("Line number:");
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(value).desired_width(120.0),
                                    );
                                    if *focus {
                                        resp.request_focus();
                                        *focus = false;
                                    }
                                });
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    if ui.button("Cancel").clicked() {
                                        close = true;
                                    }
                                    if ui.button("Go to").clicked() {
                                        if let Ok(line) = value.trim().parse::<usize>() {
                                            goto_line = Some(line.max(1));
                                        }
                                        close = true;
                                    }
                                });
                            }
                            Dialog::Font => {
                                ui.label(RichText::new("Font").size(15.0).strong());
                                ui.add_space(8.0);
                                ui.label(RichText::new("Size").size(11.0).color(p.text_weak));
                                ui.horizontal_wrapped(|ui| {
                                    for size in [
                                        9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 16.0, 18.0, 20.0, 24.0,
                                        28.0,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                (self.font_size - size).abs() < 0.1,
                                                format!("{size:.0}"),
                                            )
                                            .clicked()
                                        {
                                            self.font_size = size;
                                        }
                                    }
                                });
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "Editor font: DejaVu Sans Mono (Consolas fallback)",
                                    )
                                    .size(11.0)
                                    .color(p.text_weak),
                                );
                                ui.add_space(10.0);
                                if ui.button("Done").clicked() {
                                    close = true;
                                }
                            }
                            Dialog::About => {
                                ui.label(RichText::new("About Elevenpad").size(15.0).strong());
                                ui.add_space(6.0);
                                ui.label(format!(
                                    "Elevenpad {}, a Windows 11 Notepad reimplementation for Linux.",
                                    env!("CARGO_PKG_VERSION")
                                ));
                                ui.label(
                                    RichText::new(
                                        "Written in Rust with egui. Not affiliated with Microsoft.",
                                    )
                                    .size(11.0)
                                    .color(p.text_weak),
                                );
                                ui.add_space(10.0);
                                if ui.button("OK").clicked() {
                                    close = true;
                                }
                            }
                        }
                    });
            });

        if close {
            self.dialog = None;
        } else {
            self.dialog = Some(state);
        }
        if let Some(path) = open_path {
            self.open_path(path);
        }
        if let Some(path) = save_to {
            let idx = self.doc_index();
        let doc = &mut self.docs[idx];
            match doc.save_to(path.clone()) {
                Ok(()) => self.flash(format!("Saved {}", path.display())),
                Err(err) => self.flash(format!("Save failed: {err}")),
            }
        }
        if let Some(line) = goto_line {
            let idx = self.doc_index();
        let doc = &mut self.docs[idx];
            let idx = doc.char_index_of_line(line);
            doc.scroll_to_line = Some(line);
            self.select_chars(ctx, idx, idx);
        }
    }

    fn toast(&mut self, ctx: &egui::Context) {
        let Some(msg) = self.status_msg.clone() else {
            return;
        };
        if now() - self.status_time > 3.5 {
            self.status_msg = None;
            return;
        }
        let p = self.palette;
        let screen = ctx.screen_rect();
        egui::Area::new(Id::new("toast"))
            .fixed_pos(Pos2::new(screen.center().x - 180.0, screen.bottom() - 90.0))
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                Frame::none()
                    .fill(p.menu_fill)
                    .stroke(Stroke::new(1.0, p.menu_stroke))
                    .rounding(Rounding::same(6.0))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 4.0),
                        blur: 12.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(40),
                    })
                    .inner_margin(Margin::symmetric(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).size(11.5));
                    });
            });
    }

    // -------------------------------------------------------------- shortcuts

    fn handle_shortcuts(&mut self, ctx: &egui::Context, actions: &mut Vec<Action>) {
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::T)) {
            actions.push(Action::NewTab);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::O)) {
            actions.push(Action::OpenDialog);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::S)) {
            actions.push(Action::SaveAs);
        } else if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S)) {
            actions.push(Action::Save);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F)) {
            actions.push(Action::Find);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::H)) {
            actions.push(Action::Replace);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::G)) {
            actions.push(Action::GoTo);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::W)) {
            actions.push(Action::CloseTab(self.active));
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F3)) {
            let forward = !ctx.input(|i| i.modifiers.shift);
            actions.push(Action::FindNext(forward));
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Z)) {
            actions.push(Action::Undo);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Y))
            || ctx.input_mut(|i| i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::Z))
        {
            actions.push(Action::Redo);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::A)) {
            actions.push(Action::SelectAll);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Plus))
            || ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Equals))
        {
            actions.push(Action::ZoomIn);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Minus)) {
            actions.push(Action::ZoomOut);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Num0)) {
            actions.push(Action::ZoomReset);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F5)) {
            actions.push(Action::TimeDate);
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) && self.find_visible {
            self.find_visible = false;
        }

        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(path) = file.path {
                self.open_path(path);
            }
        }

        if ctx.input(|i| i.modifiers.ctrl) {
            let scroll = ctx.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > 0.0 {
                self.zoom = (self.zoom * (1.0 + scroll * 0.0015)).clamp(0.2, 5.0);
            }
        }
    }

    pub fn open_files(&mut self, paths: Vec<PathBuf>) {
        let pristine = self.docs.len() == 1
            && self.docs[0].path.is_none()
            && self.docs[0].text.is_empty()
            && !self.docs[0].modified;
        if pristine {
            self.docs.clear();
        }
        for path in paths {
            let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());
            if let Some(index) = self
                .docs
                .iter()
                .position(|doc| doc.path.as_ref().map(|p| p.canonicalize().unwrap_or_else(|_| p.clone())) == Some(canonical.clone()))
            {
                self.active = index;
                continue;
            }
            match Doc::from_file(path) {
                Ok(doc) => self.docs.push(doc),
                Err(err) => self.flash(format!("Could not open file: {err}")),
            }
        }
        if self.docs.is_empty() {
            self.docs.push(Doc::new());
        }
        self.active = 0;
    }

    pub fn open_path(&mut self, path: PathBuf) {
        match Doc::from_file(path) {
            Ok(doc) => {
                self.docs.push(doc);
                self.active = self.docs.len() - 1;
            }
            Err(err) => self.flash(format!("Could not open file: {err}")),
        }
    }
}

// ------------------------------------------------------------------ widgets

fn item(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    enabled: bool,
    checked: Option<bool>,
    p: Palette,
) -> bool {
    let width = 250.0;
    let height = 28.0;
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), sense);
    let hovered = resp.hovered() && enabled;
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect.shrink2(Vec2::new(2.0, 0.0)), Rounding::same(4.0), p.hover);
    }
    let mut left = rect.left() + 10.0;
    if let Some(on) = checked {
        if on {
            let c = Pos2::new(rect.left() + 12.0, rect.center().y);
            painter.line_segment(
                [c + Vec2::new(-4.0, 0.0), c + Vec2::new(-1.0, 3.0)],
                Stroke::new(1.4, p.accent),
            );
            painter.line_segment(
                [c + Vec2::new(-1.0, 3.0), c + Vec2::new(4.5, -3.5)],
                Stroke::new(1.4, p.accent),
            );
        }
        left = rect.left() + 28.0;
    }
    painter.text(
        Pos2::new(left, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        if enabled { p.text } else { p.text_disabled },
    );
    if !shortcut.is_empty() {
        painter.text(
            Pos2::new(rect.right() - 10.0, rect.center().y),
            Align2::RIGHT_CENTER,
            shortcut,
            FontId::proportional(11.5),
            p.text_disabled,
        );
    }
    resp.clicked()
}

fn separator(ui: &mut egui::Ui, p: Palette) {
    ui.add_space(3.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(250.0, 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, p.stroke),
    );
    ui.add_space(3.0);
}

fn formatting_button(ui: &mut egui::Ui, p: Palette) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 28.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, Rounding::same(4.0), p.hover);
    }
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "Aa",
        FontId::proportional(13.0),
        if resp.hovered() { p.text } else { p.text_weak },
    );
    resp
}

fn cowriter_button(ui: &mut egui::Ui, p: Palette) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 28.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, Rounding::same(4.0), p.hover);
    }
    icons::sparkle(ui, rect.center(), p.accent, 7.0);
    resp
}

fn settings_button(ui: &mut egui::Ui, p: Palette) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(30.0, 28.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, Rounding::same(4.0), p.hover);
    }
    let c = rect.center();
    let painter = ui.painter();
    painter.circle_stroke(c, 5.0, Stroke::new(1.2, p.text_weak));
    painter.circle_filled(c, 1.8, p.text_weak);
    for i in 0..6 {
        let a = i as f32 * std::f32::consts::TAU / 6.0;
        let dir = Vec2::new(a.cos(), a.sin());
        painter.line_segment([c + dir * 6.2, c + dir * 8.0], Stroke::new(1.4, p.text_weak));
    }
    resp
}

fn avatar(ui: &mut egui::Ui, p: Palette) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(28.0, 28.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 11.0, p.accent);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        "M",
        FontId::new(11.5, egui::FontFamily::Name(UI_FONT_SEMIBOLD.into())),
        Color32::WHITE,
    );
}

pub(crate) fn file_row(ui: &mut egui::Ui, name: &str, is_dir: bool, selected: bool, p: Palette) -> bool {
    let height = 26.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    if selected {
        ui.painter().rect_filled(rect, Rounding::same(4.0), p.selection);
    } else if response.hovered() {
        ui.painter().rect_filled(rect, Rounding::same(4.0), p.hover);
    }
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 16.0, rect.center().y),
        Vec2::new(18.0, 18.0),
    );
    if is_dir {
        icons::folder(ui, icon_rect, Color32::from_rgb(0xE8, 0xB8, 0x4B));
    } else {
        icons::file_page(ui, icon_rect, p.surface, p.text_weak);
    }
    ui.painter().with_clip_rect(rect).text(
        Pos2::new(rect.left() + 32.0, rect.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(12.5),
        p.text,
    );
    response.clicked()
}

pub(crate) fn list_dir(dir: &PathBuf, show_hidden: bool) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|path| {
                    show_hidden
                        || path
                            .file_name()
                            .map(|name| !name.to_string_lossy().starts_with('.'))
                            .unwrap_or(true)
                })
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by_key(|p| (!p.is_dir(), p.file_name().map(|n| n.to_owned())));
    entries
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}


impl eframe::App for NotepadApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui(ctx);
    }
}


fn resize_cursor(direction: egui::viewport::ResizeDirection) -> egui::CursorIcon {
    use egui::viewport::ResizeDirection as Dir;
    match direction {
        Dir::NorthWest | Dir::SouthEast => egui::CursorIcon::ResizeNwSe,
        Dir::NorthEast | Dir::SouthWest => egui::CursorIcon::ResizeNeSw,
        Dir::East | Dir::West => egui::CursorIcon::ResizeHorizontal,
        Dir::North | Dir::South => egui::CursorIcon::ResizeVertical,
    }
}
