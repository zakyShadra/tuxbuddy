use crate::app_state::AppState;
use crate::roaming::RoamHandle;
use crate::session::SessionState;
use protocol::TerminalMeta;
use std::sync::Arc;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn respond_to_event(
    id: String,
    decision: serde_json::Value,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    if state.resolve_pending(id, decision) {
        Ok(())
    } else {
        Err(format!("no pending hook event with id {id}"))
    }
}

#[tauri::command]
pub fn get_session_state(state: State<'_, Arc<AppState>>) -> SessionState {
    state.get_session()
}

#[tauri::command]
pub fn focus_terminal(meta: TerminalMeta) {
    crate::terminal_focus::focus(&meta);
}

#[tauri::command]
pub fn preview_hook_diff() -> Result<String, String> {
    crate::hooks::installer::preview()
}

#[tauri::command]
pub fn install_hooks() -> Result<(), String> {
    crate::hooks::installer::install()
}

#[tauri::command]
pub fn uninstall_hooks() -> Result<(), String> {
    crate::hooks::installer::uninstall()
}

/// Stops the roaming loop where it stands — called from the frontend while
/// the hooks/menu panel is open (PRD §2.2/§4.4 #4's "pause/diemin"), so
/// Tux doesn't wander off out from under an open menu.
#[tauri::command]
pub fn pause_roaming(roam: State<'_, RoamHandle>) {
    roam.pause();
}

#[tauri::command]
pub fn resume_roaming(roam: State<'_, RoamHandle>) {
    roam.resume();
}
