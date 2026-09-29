// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Native Wayland makes always_on_top/set_position no-ops for security
    // reasons — GTK has no way to grant an ordinary app absolute placement.
    // Forcing GTK onto XWayland gets the working X11 code path instead.
    // Must happen before GTK initializes, so this is as early as possible
    // and only when the user hasn't already chosen a backend themselves.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() {
        std::env::set_var("GDK_BACKEND", "x11");
    }
    tuxbuddy_lib::run()
}
