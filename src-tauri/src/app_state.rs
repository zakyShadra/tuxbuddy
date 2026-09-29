use crate::session::SessionState;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio::sync::oneshot;
use uuid::Uuid;

/// Shared app state. Per `rust-async-patterns`: the `Mutex` here is only
/// ever held for quick insert/remove/read operations, never across an
/// `.await` — the actual waiting for a decision happens on the
/// `oneshot::Receiver` returned by `register_pending`, outside any lock.
#[derive(Default)]
pub struct AppState {
    session: Mutex<SessionState>,
    pending: Mutex<HashMap<Uuid, oneshot::Sender<serde_json::Value>>>,
}

impl AppState {
    pub fn set_session(&self, state: SessionState) {
        *self.session.lock().unwrap() = state;
    }

    pub fn get_session(&self) -> SessionState {
        *self.session.lock().unwrap()
    }

    /// Registers a blocking envelope's id and returns the receiver half —
    /// the caller awaits this (with its own timeout) outside the lock.
    pub fn register_pending(&self, id: Uuid) -> oneshot::Receiver<serde_json::Value> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().unwrap().insert(id, tx);
        rx
    }

    /// Resolves a pending decision by id. Returns `true` if a waiter was
    /// found (and the send at least attempted); `false` if the id is
    /// unknown or the waiter already gave up (e.g. timed out).
    pub fn resolve_pending(&self, id: Uuid, decision: serde_json::Value) -> bool {
        if let Some(tx) = self.pending.lock().unwrap().remove(&id) {
            tx.send(decision).is_ok()
        } else {
            false
        }
    }
}
