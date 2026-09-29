use serde::Serialize;
use std::process::Command;

/// Thin wrapper over `playerctl` (MPRIS control CLI) — controls whatever
/// media player(s) currently have an active MPRIS session (Spotify, a
/// browser tab playing YouTube, etc). PRD session-3: user asked for
/// notch music-player control; `playerctl` was picked over a raw D-Bus
/// `mpris` crate for speed of implementation, at the cost of requiring
/// `playerctl` installed on the system — a real packaging concern for
/// later (PRD §4.4).
///
/// Player SELECTION is deliberately left to the frontend (see
/// `notch.ts`) rather than auto-picked here: an earlier version always
/// preferred whichever player was `Playing`, which meant the moment the
/// user paused their music it'd silently flip the notch over to a
/// different player entirely — reported live as jarring. Now this module
/// just reports every player's state; `notch.ts` remembers which one the
/// user is looking at and only re-picks a default when that one
/// disappears.
#[derive(Clone, Serialize)]
pub struct PlayerInfo {
    pub id: String,
    pub playing: bool,
    pub title: String,
    pub artist: String,
}

fn list_players() -> Vec<String> {
    Command::new("playerctl")
        .arg("-l")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn run_for(player: &str, args: &[&str]) -> Option<String> {
    let mut full = vec!["-p", player];
    full.extend_from_slice(args);
    Command::new("playerctl")
        .args(&full)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

pub fn list() -> Vec<PlayerInfo> {
    list_players()
        .into_iter()
        .map(|id| {
            let status = run_for(&id, &["status"]).unwrap_or_default();
            let title = run_for(&id, &["metadata", "title"]).unwrap_or_default();
            let artist = run_for(&id, &["metadata", "artist"]).unwrap_or_default();
            PlayerInfo { id, playing: status == "Playing", title, artist }
        })
        .collect()
}

pub fn play_pause(player: &str) {
    let _ = run_for(player, &["play-pause"]);
}

pub fn next(player: &str) {
    let _ = run_for(player, &["next"]);
}

pub fn previous(player: &str) {
    let _ = run_for(player, &["previous"]);
}
