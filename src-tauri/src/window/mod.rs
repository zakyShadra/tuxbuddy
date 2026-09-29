pub mod x11;
pub mod layer_shell;

use tauri::WebviewWindow;

/// Detects the session type and applies whichever positioning strategy
/// actually works there. X11 is the confirmed-working baseline (plan M1);
/// the Wayland/layer-shell path is still a stub (plan M2 work) and just
/// logs so the limitation is visible rather than silently broken.
pub fn apply_platform_positioning(window: &WebviewWindow) {
    if is_wayland_without_override() {
        log_wayland_fallback();
        layer_shell::apply(window);
    } else {
        x11::apply(window);
    }
}

/// Must match `tauri.conf.json`'s notch window `"width"` — passed in
/// rather than read back via `window.outer_size()` because that call
/// races the window manager applying the configured size: at the point
/// `.setup()` runs, `outer_size()` was observed returning Tauri's
/// pre-layout default (810×418) instead of the configured 300×170,
/// silently miscentering the window. The pet window has the same
/// `outer_size()` race in `x11::apply`/`position_bottom_left`, but it's
/// invisible there since `roaming.rs` overwrites the position (using its
/// own, by-then-accurate `outer_size()`) within one tick of startup.
const NOTCH_WIDTH: i32 = 330;

/// Same X11/Wayland split as `apply_platform_positioning`, but for the
/// "notch" status window (fixed top-center, never roams) added in PRD
/// session 3's two-window split.
pub fn apply_notch_positioning(window: &WebviewWindow) {
    if is_wayland_without_override() {
        log_wayland_fallback();
        layer_shell::apply(window);
    } else {
        x11::apply_notch(window, NOTCH_WIDTH);
    }
}

fn is_wayland_without_override() -> bool {
    let forced_x11 = std::env::var("GDK_BACKEND").ok().as_deref() == Some("x11");
    if forced_x11 {
        println!("tuxbuddy: GDK_BACKEND=x11 override detected — forcing X11 positioning path.");
    }
    !forced_x11 && std::env::var_os("WAYLAND_DISPLAY").is_some()
}

fn log_wayland_fallback() {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    println!(
        "tuxbuddy: Wayland session detected (XDG_CURRENT_DESKTOP={desktop}) — \
         gtk-layer-shell positioning not implemented yet (plan M2), falling back \
         to a plain movable window."
    );
}
