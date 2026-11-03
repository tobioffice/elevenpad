//! Windows 11 Notepad reimplementation, windowed binary.

mod app;
mod doc;
mod icons;
mod shape;
mod syntax;
mod theme;

use std::path::PathBuf;

fn main() -> eframe::Result<()> {
    let transparent = app::default_rounded();
    let mut files: Vec<PathBuf> = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("Elevenpad {}", env!("CARGO_PKG_VERSION"));
                println!("Usage: elevenpad [FILE...]");
                println!();
                println!("Opens each FILE in a tab. Without FILE it starts with an untitled note.");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("Elevenpad {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            other if other.starts_with('-') => {}
            other => files.push(PathBuf::from(other)),
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([640.0, 420.0])
            .with_decorations(false)
            .with_transparent(transparent)
            .with_title("Elevenpad"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "Elevenpad",
        options,
        Box::new(move |cc| {
            let mut app = app::NotepadApp::new(&cc.egui_ctx);
            if !files.is_empty() {
                app.open_files(files);
            }
            Ok(Box::new(app))
        }),
    )
}
