use tauri::{Monitor, PhysicalPosition, WebviewWindow};

/// X11 baseline (plan M1): `tao`'s always-on-top + set_position work
/// reliably here, unlike native Wayland. Starts the window at the
/// bottom-left of the primary monitor — the roaming loop (`crate::roaming`,
/// spawned right after this in `lib.rs`'s setup) takes over positioning
/// from here within one tick, per PRD §2.1's bottom-edge idle roam.
///
/// Works entirely in PHYSICAL pixels (`monitor.position()`/`.size()` and
/// `window.outer_size()` are already physical — no `to_logical()`
/// conversion here). An earlier version went through `LogicalPosition`
/// via `scale_factor()`, which silently mispositioned the notch window
/// (§4.1c) on this XWayland-forced-X11 setup — the same bug existed here
/// too, just invisible, because `roaming.rs` overwrites this window's
/// position (in physical pixels) within one tick of startup anyway.
pub fn apply(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
    let _ = window.set_decorations(false);
    let _ = window.set_skip_taskbar(true);

    if let Ok(Some(monitor)) = window.primary_monitor() {
        position_bottom_left(window, &monitor);
    } else {
        eprintln!("tuxbuddy: no primary monitor found, leaving window at its default position");
    }
}

fn position_bottom_left(window: &WebviewWindow, monitor: &Monitor) {
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let Ok(win_size) = window.outer_size() else {
        return;
    };

    let x = monitor_pos.x;
    let y = monitor_pos.y + monitor_size.height as i32 - win_size.height as i32 - 8; // small gap from the bottom edge
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

/// The "notch" status window — fixed top-center of the primary monitor,
/// never roams (unlike the pet window in `apply`/`position_bottom_left`).
/// Coucou-style menu-bar placement, per PRD's session-3 decision to split
/// reactive state animations out of the roaming pet.
pub fn apply_notch(window: &WebviewWindow, known_width: i32) {
    let _ = window.set_always_on_top(true);
    let _ = window.set_decorations(false);
    let _ = window.set_skip_taskbar(true);

    if let Ok(Some(monitor)) = window.primary_monitor() {
        position_top_center(window, &monitor, known_width);
    } else {
        eprintln!("tuxbuddy: no primary monitor found, leaving notch window at its default position");
    }
}

fn position_top_center(window: &WebviewWindow, monitor: &Monitor, known_width: i32) {
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();

    let x = monitor_pos.x + (monitor_size.width as i32 - known_width) / 2;
    let y = monitor_pos.y + 4; // small gap from the very top edge
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
