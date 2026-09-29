use tauri::{LogicalPosition, LogicalSize, Monitor, WebviewWindow};

/// X11 baseline (plan M1): `tao`'s always-on-top + set_position work
/// reliably here, unlike native Wayland. Starts the window at the
/// bottom-left of the primary monitor — the roaming loop (`crate::roaming`,
/// spawned right after this in `lib.rs`'s setup) takes over positioning
/// from here within one tick, per PRD §2.1's bottom-edge idle roam.
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
    let scale = monitor.scale_factor();
    let monitor_pos = monitor.position().to_logical::<f64>(scale);
    let monitor_size = monitor.size().to_logical::<f64>(scale);
    let Ok(win_size) = window.outer_size() else {
        return;
    };
    let win_size: LogicalSize<f64> = win_size.to_logical(scale);

    let x = monitor_pos.x;
    let y = monitor_pos.y + monitor_size.height - win_size.height - 8.0; // small gap from the bottom edge
    let _ = window.set_position(LogicalPosition::new(x, y));
}
