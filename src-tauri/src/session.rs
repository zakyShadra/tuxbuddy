use serde::{Deserialize, Serialize};

/// Mirrors `src/tux/stateConfig.ts`'s `SessionState` union — keep the
/// variant names identical (PascalCase both sides) so the frontend needs
/// no translation table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Working,
    Thinking,
    Searching,
    Approval,
    Question,
    Error,
    Finished,
    RateLimit,
}

impl Default for SessionState {
    fn default() -> Self {
        SessionState::Idle
    }
}
