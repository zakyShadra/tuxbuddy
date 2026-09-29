//! Shared types for the Unix-socket protocol between `tuxbuddy-hook` (the
//! binary Claude Code invokes as its hook command) and the TuxBuddy Tauri
//! app. Framing is newline-delimited JSON: one `HookEnvelope` per line from
//! hook -> app, one `DecisionMessage` per line back for blocking events.
//!
//! `event_name`/blocking classification is intentionally a small lookup
//! table here (see `is_blocking`) rather than baked into either binary,
//! because the exact Claude Code hook event names/schema are still pending
//! confirmation against a live session (see plan M0 "schema discovery").

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// How long the hook binary (and the app's router, as a safety net) will
/// wait for a decision on a blocking event before giving up. Kept a few
/// seconds under Claude Code's ~120s hook ceiling.
pub const BLOCKING_TIMEOUT_SECS: u64 = 115;

/// Best-effort terminal/session context gathered by the hook binary,
/// forwarded so the app can (a) show useful context in the UI and (b)
/// attempt to focus the originating terminal window (X11 only, see
/// `terminal_focus` in the app).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TerminalMeta {
    #[serde(default)]
    pub term_program: Option<String>,
    #[serde(default)]
    pub term_session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub pid: Option<u32>,
}

/// One line sent from `tuxbuddy-hook` to the app over the Unix socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookEnvelope {
    pub id: Uuid,
    pub event_name: String,
    pub blocking: bool,
    /// Raw JSON payload exactly as received from Claude Code on stdin.
    pub payload: serde_json::Value,
    pub meta: TerminalMeta,
}

/// One line sent back from the app to `tuxbuddy-hook`, only for envelopes
/// where `blocking == true`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionMessage {
    pub id: Uuid,
    /// Event-specific shape (e.g. `{"behavior":"allow"}` or
    /// `{"permissionDecision":"deny"}`) — deliberately untyped until the
    /// M0 schema discovery step confirms the real shape Claude Code expects
    /// per event.
    pub decision: serde_json::Value,
}

/// Known hook event names and whether they are expected to block waiting
/// for a decision.
///
/// SECOND CORRECTION (2026-09-29, later the same day): the first fix here
/// (see git history / PRD §4.1 point 6) made only "PreToolUse" blocking,
/// on the theory that it's Claude Code's real tool-permission gate. That
/// was wrong in a more disruptive way: `PreToolUse` with the installer's
/// `"matcher": "*"` fires for **every single tool call**, not just ones
/// that actually need a permission decision — confirmed by dogfooding
/// this exact app in this exact dev session, where ordinary Bash/Edit/Read
/// calls kept popping the Allow/Deny panel and flying the window to
/// center on the user's live desktop, with no real permission pending.
/// There is currently no known field in the hook payload that
/// distinguishes "Claude Code would have prompted the user anyway" from
/// "routine tool call, hook fired unconditionally" — figuring that out is
/// its own follow-up (PRD §4.4). Until then, NOTHING blocks: every event
/// is forwarded non-blocking so the mascot still reacts visually, but
/// `tuxbuddy-hook` never makes Claude Code wait on a human click.
pub fn is_blocking(_event_name: &str) -> bool {
    false
}

/// Resolves the Unix socket path: `$XDG_RUNTIME_DIR/tuxbuddy/tuxbuddy.sock`,
/// falling back to `/tmp/tuxbuddy-<uid>/tuxbuddy.sock` if `XDG_RUNTIME_DIR`
/// is unset (e.g. some minimal/non-systemd setups).
pub fn socket_path() -> PathBuf {
    let dir = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(runtime_dir) => PathBuf::from(runtime_dir).join("tuxbuddy"),
        None => {
            // SAFETY-free portable fallback: real uid via libc would need a
            // dependency, so shell out to `id -u` once at resolution time.
            let uid = std::process::Command::new("id")
                .arg("-u")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "0".to_string());
            PathBuf::from(format!("/tmp/tuxbuddy-{uid}"))
        }
    };
    dir.join("tuxbuddy.sock")
}
