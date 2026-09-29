use super::schema::HOOK_EVENTS;
use serde_json::{Map, Value};
use std::path::PathBuf;

fn settings_path() -> PathBuf {
    dirs_home().join(".claude").join("settings.json")
}

fn dirs_home() -> PathBuf {
    // Avoids pulling in the `dirs` crate for one lookup.
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

/// Absolute path to the `tuxbuddy-hook` binary. In dev, that's the sibling
/// of this app's own binary in `target/debug` (or `target/release`) — both
/// are built from the same Cargo workspace. Packaging (plan M3) will need
/// to install `tuxbuddy-hook` to a stable location (e.g.
/// `~/.local/share/tuxbuddy/bin/`) and update this accordingly.
pub fn hook_binary_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("tuxbuddy-hook")))
        .unwrap_or_else(|| PathBuf::from("tuxbuddy-hook"))
}

fn read_settings() -> Value {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| Value::Object(Map::new()))
}

fn is_ours(command: &str, hook_path: &str) -> bool {
    command == hook_path || command.ends_with("tuxbuddy-hook") || command.contains("tuxbuddy")
}

/// Builds the merged settings document (our hook entries added/replaced,
/// everything else untouched) without writing anything.
fn build_merged(existing: &Value, hook_path: &str) -> Value {
    let mut root = match existing.clone() {
        Value::Object(m) => m,
        _ => Map::new(),
    };

    let mut hooks_section = match root.remove("hooks") {
        Some(Value::Object(m)) => m,
        _ => Map::new(),
    };

    for (event, timeout) in HOOK_EVENTS {
        let mut matchers = match hooks_section.remove(*event) {
            Some(Value::Array(a)) => a,
            _ => Vec::new(),
        };

        // Idempotent: drop any matcher group that's ours (by command),
        // then re-append the current one. Never touch other tools' entries.
        matchers.retain(|matcher| {
            let is_ours_group = matcher
                .get("hooks")
                .and_then(|h| h.as_array())
                .map(|list| {
                    list.iter().any(|h| {
                        h.get("command")
                            .and_then(|c| c.as_str())
                            .map(|c| is_ours(c, hook_path))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            !is_ours_group
        });

        let mut entry = Map::new();
        entry.insert("matcher".to_string(), Value::String("*".to_string()));
        entry.insert(
            "hooks".to_string(),
            serde_json::json!([{
                "type": "command",
                "command": hook_path,
                "timeout": timeout,
            }]),
        );
        matchers.push(Value::Object(entry));
        hooks_section.insert(event.to_string(), Value::Array(matchers));
    }

    root.insert("hooks".to_string(), Value::Object(hooks_section));
    Value::Object(root)
}

/// Returns the pretty-printed merged settings.json for the frontend to
/// show the user before they confirm (`write` is a separate step).
pub fn preview() -> Result<String, String> {
    let existing = read_settings();
    let hook_path = hook_binary_path().display().to_string();
    let merged = build_merged(&existing, &hook_path);
    serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())
}

/// Backs up the current file (if any), then writes the merged settings —
/// only call after the user has confirmed the `preview()` diff.
pub fn install() -> Result<(), String> {
    let path = settings_path();
    let existing = read_settings();
    let hook_path = hook_binary_path().display().to_string();
    let merged = build_merged(&existing, &hook_path);

    if path.exists() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let backup = path.with_file_name(format!("settings.json.bak-{stamp}"));
        std::fs::copy(&path, &backup).map_err(|e| e.to_string())?;
    }

    write_atomic(&path, &merged)
}

/// Removes only our own hook entries, leaving everything else (including
/// other tools' hooks) untouched.
pub fn uninstall() -> Result<(), String> {
    let path = settings_path();
    if !path.exists() {
        return Ok(());
    }
    let existing = read_settings();
    let hook_path = hook_binary_path().display().to_string();

    let mut root = match existing {
        Value::Object(m) => m,
        _ => return Ok(()),
    };
    let Some(Value::Object(mut hooks_section)) = root.remove("hooks") else {
        return Ok(());
    };

    let mut empty_events = Vec::new();
    for (event, matchers) in hooks_section.iter_mut() {
        if let Value::Array(list) = matchers {
            list.retain(|matcher| {
                let is_ours_group = matcher
                    .get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hs| {
                        hs.iter().any(|h| {
                            h.get("command")
                                .and_then(|c| c.as_str())
                                .map(|c| is_ours(c, &hook_path))
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false);
                !is_ours_group
            });
            if list.is_empty() {
                empty_events.push(event.clone());
            }
        }
    }
    for event in empty_events {
        hooks_section.remove(&event);
    }
    root.insert("hooks".to_string(), Value::Object(hooks_section));

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup = path.with_file_name(format!("settings.json.bak-{stamp}"));
    std::fs::copy(&path, &backup).map_err(|e| e.to_string())?;

    write_atomic(&path, &Value::Object(root))
}

fn write_atomic(path: &std::path::Path, value: &Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let pretty = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, pretty).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_without_touching_unrelated_keys_and_hooks() {
        let existing: Value = serde_json::json!({
            "someOtherSetting": true,
            "hooks": {
                "PreToolUse": [
                    { "matcher": "*", "hooks": [{ "type": "command", "command": "/usr/bin/other-tool-hook", "timeout": 5 }] }
                ]
            }
        });
        let merged = build_merged(&existing, "/opt/tuxbuddy/tuxbuddy-hook");

        assert_eq!(merged["someOtherSetting"], true, "unrelated top-level key must survive");

        let pre = merged["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2, "other tool's matcher group must be kept, ours appended");
        let commands: Vec<&str> = pre
            .iter()
            .flat_map(|m| m["hooks"].as_array().unwrap())
            .map(|h| h["command"].as_str().unwrap())
            .collect();
        assert!(commands.contains(&"/usr/bin/other-tool-hook"));
        assert!(commands.contains(&"/opt/tuxbuddy/tuxbuddy-hook"));
    }

    #[test]
    fn is_idempotent_on_reinstall() {
        let existing: Value = serde_json::json!({});
        let first = build_merged(&existing, "/opt/tuxbuddy/tuxbuddy-hook");
        let second = build_merged(&first, "/opt/tuxbuddy/tuxbuddy-hook");

        let pre = second["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 1, "re-running install must replace, not duplicate, our own entry");
    }

    #[test]
    fn covers_every_planned_event() {
        let merged = build_merged(&Value::Object(Map::new()), "/opt/tuxbuddy/tuxbuddy-hook");
        let hooks = merged["hooks"].as_object().unwrap();
        for (event, _) in HOOK_EVENTS {
            assert!(hooks.contains_key(*event), "missing event: {event}");
        }
    }
}
