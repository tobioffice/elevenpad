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
    let mut use_wgpu = false;
    let mut files: Vec<PathBuf> = Vec::new();

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("Elevenpad {}", env!("CARGO_PKG_VERSION"));
                println!("Usage: elevenpad [--wgpu] [FILE...]");
                println!();
                println!("Opens each FILE in a tab. Without FILE it starts with an untitled note.");
                println!();
                println!("  --wgpu   Use the wgpu/Vulkan renderer instead of OpenGL.");
                println!("           OpenGL is the default because it resizes smoothly on");
                println!("           Wayland with NVIDIA, where the Vulkan swapchain lags.");
                println!("  --gl     Accepted for compatibility; OpenGL is already the default.");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("Elevenpad {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--wgpu" => use_wgpu = true,
            "--gl" => use_wgpu = false,
            other if other.starts_with('-') => {}
            other => files.push(PathBuf::from(other)),
        }
    }

    let preferred = if use_wgpu {
        eframe::Renderer::Wgpu
    } else {
        eframe::Renderer::Glow
    };

    match launch(preferred, files.clone(), transparent) {
        Ok(()) => Ok(()),
        Err(error) if !use_wgpu => {
            eprintln!("OpenGL renderer failed ({error}); falling back to wgpu");
            launch(eframe::Renderer::Wgpu, files, transparent)
        }
        Err(error) => Err(error),
    }
}

fn launch(
    renderer: eframe::Renderer,
    files: Vec<PathBuf>,
    transparent: bool,
) -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([640.0, 420.0])
            .with_decorations(false)
            .with_transparent(transparent)
            .with_title("Elevenpad"),
        renderer,
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
