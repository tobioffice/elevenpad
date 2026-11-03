//! X11 shape mask for rounded corners without relying on compositor alpha.
//!
//! On Wayland the window is shaped by its alpha channel. On X11 (including
//! XWayland) a compositing manager may be off or may hand the window an opaque
//! visual, which turns the unpainted corners black. The X Shape extension cuts
//! the window itself, so the corners show whatever is behind it either way.

#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(target_os = "linux")]
static BUSY: AtomicBool = AtomicBool::new(false);

#[cfg(target_os = "linux")]
pub fn apply(logical_width: f32, logical_height: f32, radius: f32) {
    if std::env::var_os("DISPLAY").is_none() {
        return;
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return;
    }
    let width = logical_width.round().max(1.0) as u16;
    let height = logical_height.round().max(1.0) as u16;
    let radius = radius.max(0.0) as u16;
    std::thread::spawn(move || {
        let _ = apply_inner(width, height, radius);
        BUSY.store(false, Ordering::SeqCst);
    });
}

#[cfg(not(target_os = "linux"))]
pub fn apply(_logical_width: f32, _logical_height: f32, _radius: f32) {}

#[cfg(target_os = "linux")]
fn apply_inner(
    logical_width: u16,
    logical_height: u16,
    radius: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    use x11rb::connection::Connection;
    use x11rb::protocol::shape::{self, ConnectionExt as _};
    use x11rb::protocol::xproto::{self, *};

    let (conn, screen_num) = x11rb::connect(None)?;
    let root = conn.setup().roots[screen_num].root;

    let pid_atom = conn.intern_atom(false, b"_NET_WM_PID")?.reply()?.atom;
    let tree = conn.query_tree(root)?.reply()?;
    let mut target = None;
    for &window in &tree.children {
        let property = conn
            .get_property(false, window, pid_atom, AtomEnum::CARDINAL, 0, 1)?
            .reply()?;
        if property.value.len() >= 4 {
            let pid = u32::from_ne_bytes([
                property.value[0],
                property.value[1],
                property.value[2],
                property.value[3],
            ]);
            if pid == std::process::id() {
                target = Some(window);
                break;
            }
        }
    }
    let Some(window) = target else {
        return Ok(());
    };

    let geometry = conn.get_geometry(window)?.reply()?;
    let width = geometry.width;
    let height = geometry.height;
    if width == 0 || height == 0 {
        return Ok(());
    }
    let scale = width as f32 / logical_width.max(1) as f32;
    let radius_px = ((radius as f32) * scale).round().clamp(0.0, (width.min(height) / 2) as f32) as i16;

    let pixmap = conn.generate_id()?;
    conn.create_pixmap(1, pixmap, root, width, height)?;
    let gc = conn.generate_id()?;
    conn.create_gc(gc, pixmap, &CreateGCAux::new().foreground(0))?;
    conn.poly_fill_rectangle(
        pixmap,
        gc,
        &[Rectangle {
            x: 0,
            y: 0,
            width,
            height,
        }],
    )?;
    conn.change_gc(gc, &ChangeGCAux::new().foreground(1))?;
    let points = rounded_points(width as i16, height as i16, radius_px);
    xproto::fill_poly(
        &conn,
        pixmap,
        gc,
        PolyShape::CONVEX,
        CoordMode::ORIGIN,
        &points,
    )?;
    conn.shape_mask(shape::SO::SET, shape::SK::BOUNDING, window, 0, 0, pixmap)?;

    conn.free_gc(gc)?;
    conn.free_pixmap(pixmap)?;
    conn.flush()?;
    let _ = logical_height;
    Ok(())
}

#[cfg(target_os = "linux")]
fn rounded_points(width: i16, height: i16, radius: i16) -> Vec<x11rb::protocol::xproto::Point> {
    use x11rb::protocol::xproto::Point;
    if radius <= 0 {
        return vec![
            Point { x: 0, y: 0 },
            Point { x: width, y: 0 },
            Point { x: width, y: height },
            Point { x: 0, y: height },
        ];
    }
    let steps = 8;
    let corners = [
        (radius, radius, 180.0_f32),
        (width - radius, radius, 270.0),
        (width - radius, height - radius, 0.0),
        (radius, height - radius, 90.0),
    ];
    let mut points = Vec::with_capacity(4 * (steps + 1));
    for (cx, cy, start) in corners {
        for step in 0..=steps {
            let angle = (start + 90.0 * step as f32 / steps as f32).to_radians();
            points.push(Point {
                x: (cx as f32 + radius as f32 * angle.cos()).round() as i16,
                y: (cy as f32 + radius as f32 * angle.sin()).round() as i16,
            });
        }
    }
    points
}
