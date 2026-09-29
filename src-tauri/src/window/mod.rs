pub mod x11;
pub mod layer_shell;

use tauri::WebviewWindow;

/// Detects the session type and applies whichever positioning strategy
/// actually works there. X11 is the confirmed-working baseline (plan M1);
/// the Wayland/layer-shell path is still a stub (plan M2 work) and just
/// logs so the limitation is visible rather than silently broken.
pub fn apply_platform_positioning(window: &WebviewWindow) {
    let forced_x11 = std::env::var("GDK_BACKEND").ok().as_deref() == Some("x11");
    let is_wayland = !forced_x11 && std::env::var_os("WAYLAND_DISPLAY").is_some();
    if forced_x11 {
        println!("tuxbuddy: GDK_BACKEND=x11 override detected — forcing X11 positioning path.");
    }
    if is_wayland {
        let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
        println!(
            "tuxbuddy: Wayland session detected (XDG_CURRENT_DESKTOP={desktop}) — \
             gtk-layer-shell positioning not implemented yet (plan M2), falling back \
             to a plain movable window."
        );
        layer_shell::apply(window);
    } else {
        x11::apply(window);
    }
}
