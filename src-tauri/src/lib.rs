mod app_state;
mod commands;
mod hooks;
mod roaming;
mod session;
mod socket;
mod terminal_focus;
mod window;

use app_state::AppState;
use std::sync::Arc;
use tauri::Manager;

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
        ])
        .setup(|app| {
            let handle = app.handle();
            socket::server::spawn(handle);

            if let Some(window) = app.get_webview_window("main") {
                window::apply_platform_positioning(&window);
            }

            let roam_handle = roaming::spawn(handle.clone());
            app.manage(roam_handle);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
