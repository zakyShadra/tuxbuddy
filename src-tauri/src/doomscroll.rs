use std::process::Command;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// PRD session-3 behavior #3 (first pass): nudge the user when a
/// doomscroll-prone site looks like it's open, via a placeholder "tap the
/// screen" gesture — user's own words: "klo doom scroll kasih animasi
/// ketuk layar aja dulu, nanti aku kasih animasi yang sesuai" (no real
/// footage for this yet, a proper "marah"/annoyed clip comes later once
/// generated — see PRD §4.4).
///
/// Deliberately does NOT try to detect the *focused/active* window —
/// `xdotool getactivewindow` returns an id but an empty name on this
/// session (same underlying gap as the desktop-state one `kwin.rs`
/// documents: XWayland doesn't bridge every bit of Wayland-native state
/// back to X11 EWMH). Instead this just checks whether a window with a
/// matching title exists at all (`wmctrl -l`, which *has* proven
/// reliable for enumerating windows on the current desktop throughout
/// this project) — a cheaper, more robust signal than trying to fix
/// active-window tracking first. Known tradeoff: a matching tab sitting
/// open-but-unfocused in the background still counts; there's no
/// duration/"actually scrolling" threshold yet either (PRD §4.4).
const POLL_MS: u64 = 5000;
/// Minimum gap between taps so an open tab doesn't get nagged every poll.
const COOLDOWN: Duration = Duration::from_secs(90);

const SITE_KEYWORDS: [&str; 6] = ["tiktok", "instagram", "reels", "shorts", "facebook", "x.com"];
const OWN_WINDOW_TITLES: [&str; 2] = ["TuxBuddy Notch", "TuxBuddy"];

fn matching_window_open() -> bool {
    let Ok(out) = Command::new("wmctrl").arg("-l").output() else {
        return false;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        // wmctrl -l columns: <win-id> <desktop> <host> <title...>
        let Some(title) = line.splitn(4, char::is_whitespace).nth(3) else {
            continue;
        };
        if OWN_WINDOW_TITLES.iter().any(|t| title.contains(t)) {
            continue;
        }
        let lower = title.to_lowercase();
        if SITE_KEYWORDS.iter().any(|k| lower.contains(k)) {
            return true;
        }
    }
    false
}

pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(POLL_MS));
        let mut last_tap: Option<Instant> = None;

        loop {
            interval.tick().await;
            if !matching_window_open() {
                continue;
            }
            let now = Instant::now();
            if last_tap.is_some_and(|t| now.duration_since(t) < COOLDOWN) {
                continue;
            }
            last_tap = Some(now);
            let _ = app.emit_to("main", "tuxbuddy://tap", ());
        }
    });
}
