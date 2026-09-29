mod app_state;
mod commands;
mod desktop_follow;
mod doomscroll;
mod hooks;
mod kwin;
mod media;
mod roaming;
mod session;
mod socket;
mod sticky;
mod terminal_focus;
mod window;

use app_state::AppState;
use std::sync::Arc;
use tauri::{Manager, WebviewWindow, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            commands::respond_to_event,
            commands::get_session_state,
            commands::focus_terminal,
            commands::preview_hook_diff,
            commands::install_hooks,
            commands::uninstall_hooks,
            commands::pause_roaming,
            commands::resume_roaming,
            commands::nudge_pet,
            commands::media_list,
            commands::media_play_pause,
            commands::media_next,
            commands::media_previous,
        ])
        .setup(|app| {
            let handle = app.handle();
            socket::server::spawn(handle);

            // Auto-install on every startup (PRD session-3: "buat jadi
            // default aja" — no more manual Install button in the UI).
            // Guarded by is_installed() so a no-op launch doesn't spam a
            // new settings.json.bak-<timestamp> file every single time.
            if !hooks::installer::is_installed() {
                match hooks::installer::install() {
                    Ok(()) => println!("tuxbuddy: hooks auto-installed to ~/.claude/settings.json"),
                    Err(e) => eprintln!("tuxbuddy: auto-install hooks failed: {e}"),
                }
            }

            if let Some(window) = app.get_webview_window("main") {
                window::apply_platform_positioning(&window);
                prevent_close(&window);
            }
            if let Some(notch) = app.get_webview_window("notch") {
                window::apply_notch_positioning(&notch);
                prevent_close(&notch);
            }

            let roam_handle = roaming::spawn(handle.clone());
            desktop_follow::spawn(handle.clone(), roam_handle.clone());
            app.manage(roam_handle);
            sticky::spawn_apply();
            doomscroll::spawn(handle.clone());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// PRD session-3: "gabisa aku close dan gabisaa ku close... ngga kayak
/// aplikasi yang berjalan" — TuxBuddy is meant to be a persistent fixture,
/// not a normal window a user (or an errant Alt+F4/window-manager close
/// action) can dismiss. Swallows every `CloseRequested` so the window
/// only ever goes away with the whole process (killing `tuxbuddy`
/// itself, or — while running under `tauri dev` — the dev server).
/// That process-level exit is a separate, real limitation of running in
/// dev mode rather than as an installed/packaged app (PRD §4.4 #10) —
/// this only closes the gap of the window itself being closable while
/// the process is alive.
fn prevent_close(window: &WebviewWindow) {
    window.on_window_event(|event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
        }
    });
}
