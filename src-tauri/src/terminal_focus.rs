use protocol::TerminalMeta;

/// Best-effort "jump to the terminal that triggered this event" — X11 only
/// (via `wmctrl`). On Wayland this is a known no-op; `wmctrl`/`xdotool`
/// don't see native Wayland windows. Never treated as a hard requirement:
/// failing here just means the click did nothing.
pub fn focus(meta: &TerminalMeta) {
    let Some(pid) = meta.pid else { return };
    if try_focus_by_pid(pid) {
        return;
    }
    // No class-name fallback yet (plan M2) — most terminals report a
    // usable PID via wmctrl's `-p` column, so this covers the common case.
}

fn try_focus_by_pid(pid: u32) -> bool {
    let Ok(output) = std::process::Command::new("wmctrl").arg("-l").arg("-p").output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    // wmctrl -l -p columns: <win-id> <desktop> <pid> <host> <title...>
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() >= 3 && cols[2].parse::<u32>() == Ok(pid) {
            let win_id = cols[0];
            return std::process::Command::new("wmctrl")
                .arg("-i")
                .arg("-a")
                .arg(win_id)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        }
    }
    false
}
