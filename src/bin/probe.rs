//! Headless interaction probe: focus and window-drag checks without a display.

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

use egui::{Event, PointerButton, Pos2, RawInput, Rect};

fn frame(ctx: &egui::Context, app: &mut app::NotepadApp, time: f64, events: Vec<Event>) -> egui::FullOutput {
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1040.0, 700.0))),
        time: Some(time),
        events,
        ..Default::default()
    };
    ctx.run(input, |ctx| app.ui(ctx))
}

fn main() {
    let ctx = egui::Context::default();
    ctx.set_pixels_per_point(1.0);
    let mut app = app::NotepadApp::new(&ctx);
    app.docs[0].text = "hello\nworld\n".into();

    let tmp = std::env::temp_dir().join("elevenpad-open-test.txt");
    std::fs::write(&tmp, "opened from the file manager\n").unwrap();
    let mut opener = app::NotepadApp::new(&ctx);
    opener.open_files(vec![tmp.clone()]);
    println!(
        "open_files -> docs={} title={:?} text={:?}",
        opener.docs.len(),
        opener.docs[0].title(),
        opener.docs[0].text
    );
    std::fs::remove_file(&tmp).ok();

    let editor = egui::Id::new("notepad-editor");
    let cases = [
        ("editor middle", Pos2::new(600.0, 400.0)),
        ("editor lower", Pos2::new(300.0, 600.0)),
        ("editor right", Pos2::new(900.0, 200.0)),
        ("tab strip", Pos2::new(120.0, 20.0)),
    ];

    for (name, pos) in cases {
        frame(&ctx, &mut app, 0.0, vec![Event::PointerMoved(pos)]);
        frame(
            &ctx,
            &mut app,
            0.016,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        frame(
            &ctx,
            &mut app,
            0.033,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        let focused = ctx.memory(|m| m.focused());
        println!(
            "click {name:14} at {pos:?} -> focused={:?} editor={}",
            focused,
            focused == Some(editor)
        );
    }

    for (name, pos) in [
        ("empty strip", Pos2::new(700.0, 20.0)),
        ("empty strip 2", Pos2::new(860.0, 20.0)),
        ("on a tab", Pos2::new(120.0, 20.0)),
    ] {
        frame(&ctx, &mut app, 1.0, vec![Event::PointerMoved(pos)]);
        let out = frame(
            &ctx,
            &mut app,
            1.016,
            vec![
                Event::PointerMoved(pos),
                Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let mut commands = Vec::new();
        for (_, viewport) in &out.viewport_output {
            commands.extend(viewport.commands.iter().map(|c| format!("{c:?}")));
        }
        println!("drag {name:14} at {pos:?} -> {commands:?}");
        frame(
            &ctx,
            &mut app,
            1.033,
            vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
    }
}
