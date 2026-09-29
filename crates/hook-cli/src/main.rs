//! `tuxbuddy-hook` — the binary Claude Code invokes as the command for every
//! configured hook event. Reads the hook's JSON payload from stdin, forwards
//! it to the running TuxBuddy app over a Unix socket, and — for events
//! classified as blocking — waits for the app's decision and prints it to
//! stdout for Claude Code to consume.
//!
//! Golden rule: a bug here must never hang or break the user's Claude Code
//! session. Every failure path below fails open (exit 0, minimal or no
//! output) rather than erroring loudly.

use protocol::{is_blocking, socket_path, DecisionMessage, HookEnvelope, TerminalMeta, BLOCKING_TIMEOUT_SECS};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Duration;
use uuid::Uuid;

fn main() {
    // Never let a panic here propagate as a hook failure that could block
    // Claude Code — catch it and exit 0 instead.
    let _ = std::panic::catch_unwind(run).map_err(|_| {
        std::process::exit(0);
    });
}

fn run() {
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() || raw.trim().is_empty() {
        return;
    }

    let payload: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => return,
    };

    let event_name = payload
        .get("hook_event_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let blocking = is_blocking(&event_name);

    let meta = TerminalMeta {
        term_program: std::env::var("TERM_PROGRAM").ok(),
        term_session_id: std::env::var("TERM_SESSION_ID").ok(),
        cwd: payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| std::env::current_dir().ok().map(|p| p.display().to_string())),
        pid: Some(std::process::id()),
    };

    let envelope = HookEnvelope {
        id: Uuid::new_v4(),
        event_name,
        blocking,
        payload,
        meta,
    };

    let Ok(mut stream) = UnixStream::connect(socket_path()) else {
        // App isn't running (or socket is stale) — fail open, say nothing.
        return;
    };

    let Ok(line) = serde_json::to_string(&envelope) else {
        return;
    };
    if stream.write_all(line.as_bytes()).is_err() || stream.write_all(b"\n").is_err() {
        return;
    }
    let _ = stream.flush();

    if !blocking {
        // Fire-and-forget: don't hold the connection open waiting for a
        // response that will never come.
        return;
    }

    let _ = stream.set_read_timeout(Some(Duration::from_secs(BLOCKING_TIMEOUT_SECS)));
    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    if reader.read_line(&mut response_line).is_err() || response_line.trim().is_empty() {
        // Timed out, or app closed the connection without answering —
        // fail open (no decision printed => Claude Code falls back to its
        // own default behavior for this event).
        return;
    }

    if let Ok(msg) = serde_json::from_str::<DecisionMessage>(&response_line) {
        if let Ok(out) = serde_json::to_string(&msg.decision) {
            println!("{out}");
        }
    }
}
