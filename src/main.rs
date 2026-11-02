//! Windows 11 Notepad reimplementation, windowed binary.

mod app;
mod doc;
mod icons;
mod theme;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([640.0, 420.0])
            .with_decorations(false)
            .with_title("Notepad"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Notepad",
        options,
        Box::new(|cc| Ok(Box::new(app::NotepadApp::new(&cc.egui_ctx)))),
    )
}
