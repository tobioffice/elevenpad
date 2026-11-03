//! Render the UI offscreen to a PNG, for headless verification.

#[path = "../app.rs"]
mod app;
#[path = "../doc.rs"]
mod doc;
#[path = "../icons.rs"]
mod icons;
#[path = "../syntax.rs"]
mod syntax;
#[path = "../theme.rs"]
mod theme;

use egui::Rect;

fn parse_pos(arg: &str, prefix: &str) -> Option<egui::Pos2> {
    let value = arg.strip_prefix(prefix)?;
    let (x, y) = value.split_once(',')?;
    Some(egui::pos2(x.parse().ok()?, y.parse().ok()?))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "screenshot.png".to_string());
    let dark = args.iter().any(|a| a == "--dark");
    let show_find = args.iter().any(|a| a == "--find");
    let show_syntax = args.iter().any(|a| a == "--syntax");
    let show_save = args.iter().any(|a| a == "--save-dialog");
    let show_rows = args.iter().any(|a| a == "--rows");
    let hover_at = args.iter().find_map(|a| parse_pos(a, "--hover-at="));
    let resize_test = args.iter().any(|a| a == "--resize-test");
    let press_at = args.iter().find_map(|a| parse_pos(a, "--press-at="));
    let click_at = args.iter().find_map(|a| parse_pos(a, "--click-at="));
    let resize_at = args
        .iter()
        .find_map(|a| a.strip_prefix("--resize-at="))
        .and_then(|v| {
            let (x, y) = v.split_once(',')?;
            Some(egui::pos2(x.parse().ok()?, y.parse().ok()?))
        })
        .unwrap_or(egui::pos2(2.0, 300.0));
    let width = 1040u32;
    let height = 700u32;

    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = app::NotepadApp::new(&ctx);
    if dark {
        app.dark = true;
        app.palette = theme::Palette::dark();
        ctx.set_style(theme::style(app.palette, true));
    }
    app.docs[0].text = String::from(
        "Windows 11 Notepad, reimplemented in Rust.\n\
         \n\
         This window is rendered offscreen with egui and wgpu, so it can be\n\
         verified on a headless machine.\n\
         \n\
         Try the tabs, the menus, Ctrl+F for Find, and Ctrl+H for Replace.\n",
    );
    if show_syntax {
        app.docs[0].path = Some(std::path::PathBuf::from("example.rs"));
        app.docs[0].text = String::from(
            "use std::fs;\n\n/// Count lines in a file.\npub fn count_lines(path: &str) -> usize {\n    let text = fs::read_to_string(path).unwrap_or_default();\n    text.lines().count()\n}\n\nfn main() {\n    let n = count_lines(\"notes.txt\");\n    println!(\"{} lines\", n); // report\n}\n",
        );
    }
    app.docs[0].scroll_to_line = Some(1);
    let mut second = doc::Doc::new();
    second.path = Some(std::path::PathBuf::from("notes.txt"));
    second.text = "Groceries\n- coffee\n- milk\n".into();
    second.modified = true;
    app.docs.push(second);
    if show_save {
        app.save(true);
    }
    if show_find {
        app.find_visible = true;
        app.find.query = "Notepad".into();
        app.find.message = Some("2 matches".into());
    }

    let move_event = hover_at.map(egui::Event::PointerMoved);
    let mut raw_input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(width as f32, height as f32),
        )),
        events: move_event.into_iter().collect(),
        ..Default::default()
    };
    if let Some(pos) = press_at {
        raw_input.events.push(egui::Event::PointerMoved(pos));
        raw_input.events.push(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        });
    }
    if let Some(pos) = click_at {
        raw_input.events.push(egui::Event::PointerMoved(pos));
        for pressed in [true, false] {
            raw_input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            });
        }
    }
    if resize_test {
        raw_input
            .events
            .push(egui::Event::PointerMoved(resize_at));
        raw_input.events.push(egui::Event::PointerButton {
            pos: resize_at,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        });
    }

    if hover_at.is_some() {
        let mut warm = raw_input.clone();
        warm.time = Some(0.0);
        let _ = ctx.run(warm, |ctx| {
            app.ui(ctx);
        });
        raw_input.time = Some(1.0 / 60.0);
    }
    let output = ctx.run(raw_input, |ctx| {
        if show_rows {
            egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(app.palette.surface))
                .show(ctx, |ui| {
                    ui.add_space(30.0);
                    ui.set_width(320.0);
                    let p = app.palette;
                    let items = [
                        ("Documents", true),
                        ("Projects", true),
                        ("assets", true),
                        ("notes.txt", false),
                        ("main.rs", false),
                        ("README.md", false),
                    ];
                    for (i, (name, dir)) in items.iter().enumerate() {
                        app::file_row(ui, name, *dir, i == 1, p);
                    }
                });
        } else {
            app.ui(ctx);
        }
    });

    if resize_test || press_at.is_some() {
        for (id, viewport) in &output.viewport_output {
            println!("viewport {:?} commands: {:?}", id, viewport.commands);
        }
    }
    if click_at.is_some() {
        let editor = egui::Id::new("notepad-editor");
        let focused = ctx.memory(|m| m.focused());
        println!("focused={:?} editor={:?} match={}", focused, editor, focused == Some(editor));
    }
    let pixels_per_point = output.pixels_per_point;
    let paint_jobs = ctx.tessellate(output.shapes, pixels_per_point);

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("no wgpu adapter (lavapipe missing?)");
    eprintln!("adapter: {:?}", adapter.get_info());

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("notepad-offscreen"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
        },
        None,
    ))
    .expect("device");

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("frame"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
    for (id, delta) in &output.textures_delta.set {
        renderer.update_texture(&device, &queue, *id, delta);
    }

    let screen_descriptor = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point,
    };
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("encoder"),
    });
    let user_cmd_buffers =
        renderer.update_buffers(&device, &queue, &mut encoder, &paint_jobs, &screen_descriptor);
    {
        let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(if dark {
                        wgpu::Color { r: 0.09, g: 0.10, b: 0.13, a: 1.0 }
                    } else {
                        wgpu::Color { r: 0.80, g: 0.84, b: 0.90, a: 1.0 }
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let mut pass = pass.forget_lifetime();
        renderer.render(&mut pass, &paint_jobs, &screen_descriptor);
    }

    let bytes_per_pixel = 4u32;
    let unpadded = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: (padded * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(user_cmd_buffers.into_iter().chain(std::iter::once(encoder.finish())));

    let slice = buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().expect("map").expect("map failed");
    let data = slice.get_mapped_range();

    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in 0..height {
        let start = (row * padded) as usize;
        let end = start + unpadded as usize;
        pixels.extend_from_slice(&data[start..end]);
    }
    drop(data);
    buffer.unmap();

    image::save_buffer(
        &out,
        &pixels,
        width,
        height,
        image::ColorType::Rgba8,
    )
    .expect("write png");
    println!("wrote {out}");
}
