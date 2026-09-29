use tauri::WebviewWindow;

/// STUB — Wayland positioning via `gtk-layer-shell` is plan M2 work (see
/// the project plan's "Floating bar & positioning Wayland/X11" section).
/// For now we just make the window a normal, user-movable floating window
/// so the app is still usable on Wayland, rather than mispositioned or
/// crashing. `gtk-layer-shell` is already installed on this dev machine
/// (KDE Plasma Wayland) ready for the real implementation.
pub fn apply(window: &WebviewWindow) {
    let _ = window.set_decorations(true);
    let _ = window.set_skip_taskbar(false);
    // Deliberately no always_on_top/set_position here: on native Wayland
    // `tao` silently no-ops those, so pretending they worked would be
    // misleading. Leave the window as a plain, draggable window instead.
}
