use crate::app_state::AppState;
use crate::roaming::RoamHandle;
use crate::session::SessionState;
use protocol::{DecisionMessage, HookEnvelope, BLOCKING_TIMEOUT_SECS};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Handles exactly one `tuxbuddy-hook` connection: reads one JSON
/// envelope line, updates shared session state, forwards it to the
/// frontend, and — for blocking events — waits (with a timeout matching
/// the hook binary's own margin) for the frontend's decision before
/// writing it back. If nothing answers in time, the connection is simply
/// dropped: `tuxbuddy-hook` has its own identical timeout and fails open,
/// so this is a safety net, not the primary mechanism.
pub async fn handle_connection(stream: UnixStream, app: AppHandle, state: Arc<AppState>) {
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    if let Err(e) = reader.read_line(&mut line).await {
        eprintln!("tuxbuddy: socket read error: {e}");
        return;
    }
    if line.trim().is_empty() {
        return;
    }

    let envelope: HookEnvelope = match serde_json::from_str(&line) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("tuxbuddy: bad envelope JSON: {e}");
            return;
        }
    };

    if let Some(next) = map_event_to_state(&envelope.event_name) {
        state.set_session(next);
    }

    let _ = app.emit("tuxbuddy://hook-event", &envelope);

    if !envelope.blocking {
        return;
    }

    // Fly the window itself to center for the duration of the decision —
    // not just the sprite's fly-in animation (PRD §2.1/§4.4 #2). Resume
    // roaming afterward regardless of how the wait ends (answered or timed
    // out) so Tux never stays frozen mid-screen.
    if let Some(roam) = app.try_state::<RoamHandle>() {
        roam.fly_to_center();
    }

    let rx = state.register_pending(envelope.id);
    let decision = tokio::time::timeout(Duration::from_secs(BLOCKING_TIMEOUT_SECS), rx).await;

    if let Some(roam) = app.try_state::<RoamHandle>() {
        roam.resume();
    }

    let Ok(Ok(decision)) = decision else {
        // Timed out, or the sender was dropped without answering — let
        // tuxbuddy-hook's own timeout handle the fail-open behavior.
        return;
    };

    let msg = DecisionMessage { id: envelope.id, decision };
    if let Ok(mut out) = serde_json::to_string(&msg) {
        out.push('\n');
        let _ = write_half.write_all(out.as_bytes()).await;
    }
}

/// PROVISIONAL — mirrors `src/ipc/tauriBridge.ts`'s `eventNameToState`.
/// Keep the two in sync; neither is authoritative until the M0 schema
/// discovery step confirms real Claude Code event names against a live
/// session. This copy only feeds `get_session_state`/backend-side state;
/// the frontend maps independently for rendering so a mismatch here never
/// breaks the mascot's own reaction to events, only the queryable state.
fn map_event_to_state(event_name: &str) -> Option<SessionState> {
    match event_name {
        "SessionStart" => Some(SessionState::Idle),
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => Some(SessionState::Working),
        "PermissionRequest" => Some(SessionState::Approval),
        "Notification" => Some(SessionState::Question),
        "PostToolUseFailure" | "StopFailure" => Some(SessionState::Error),
        "Stop" | "SessionEnd" => Some(SessionState::Finished),
        "SubagentStart" => Some(SessionState::Thinking),
        "SubagentStop" => Some(SessionState::Finished),
        _ => None,
    }
}
