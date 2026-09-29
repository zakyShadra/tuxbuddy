use crate::kwin;
use crate::roaming::RoamHandle;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

/// Polling interval for virtual-desktop switches — no D-Bus *signal*
/// subscription here (would need a proper client like the `zbus` crate;
/// this shells out instead, consistent with the rest of the app), so this
/// just asks KWin every half-second via `qdbus6`. Cheap enough (one
/// property read) not to matter at this cadence.
const POLL_MS: u64 = 500;

/// Makes the PET window (only — the notch stays simply sticky, see
/// `sticky.rs`) actually follow the user across virtual desktops with an
/// entrance flourish, instead of being visible everywhere at once (PRD
/// session-3: "gada cara lain biar tux bawah tuh ngga sticky lagi
/// following workspace user?" — sticky was the first pass for *both*
/// windows; this splits them so the pet's "desktop pet" framing — a
/// character that's really somewhere, not a fixed status icon — holds up).
///
/// On every detected desktop switch: move the pet onto the new current
/// desktop via KWin scripting (`window.desktops = [workspace.currentDesktop]`,
/// confirmed live to actually make the window disappear from the old
/// desktop and appear on the new one — this is a real move, not sticky),
/// pause roaming, and emit `tuxbuddy://entrance` so `renderPet.ts`'s
/// `playEntrance` (descend -> run -> jump) plays instead of the window
/// just silently popping into place. The frontend calls `resume_roaming`
/// itself once the sequence finishes (same pattern as the hooks/WASD menu
/// toggles).
pub fn spawn(app: AppHandle, roam: RoamHandle) {
    tauri::async_runtime::spawn(async move {
        // Give sticky.rs's startup script (and the window itself) time to
        // be mapped before the first poll.
        tokio::time::sleep(Duration::from_millis(800)).await;

        // Seeded from the current desktop at startup so the very first
        // tick never fires a spurious entrance.
        let mut last = kwin::current_desktop();
        let mut interval = tokio::time::interval(Duration::from_millis(POLL_MS));

        loop {
            interval.tick().await;
            let Some(cur) = kwin::current_desktop() else { continue };
            if last.as_deref() == Some(cur.as_str()) {
                continue;
            }
            last = Some(cur.clone());

            let script = r#"
var wins = workspace.windowList();
for (var i = 0; i < wins.length; i++) {
    var w = wins[i];
    if (w.caption === "TuxBuddy") {
        w.desktops = [workspace.currentDesktop];
    }
}
"#;
            if kwin::run_script(script, "tuxbuddy-follow") {
                roam.pause();
                let _ = app.emit_to("main", "tuxbuddy://entrance", ());
            }
        }
    });
}
