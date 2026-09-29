/// We deliberately work with raw `serde_json::Value` for the user's
/// `~/.claude/settings.json` rather than a strict struct — the file may
/// contain keys/fields we don't model (other tools' hooks, unrelated
/// settings), and a strict struct would silently drop them on
/// round-trip. See `installer.rs`.
pub const HOOK_EVENTS: &[(&str, u64)] = &[
    ("SessionStart", 10),
    ("SessionEnd", 10),
    ("UserPromptSubmit", 120),
    ("PreToolUse", 120),
    ("PostToolUse", 10),
    ("PostToolUseFailure", 10),
    ("PermissionRequest", 120),
    ("Notification", 10),
    ("Stop", 120),
    ("StopFailure", 10),
    ("SubagentStart", 10),
    ("SubagentStop", 10),
];
