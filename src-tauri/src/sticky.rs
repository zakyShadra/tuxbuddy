use crate::kwin;
use std::time::Duration;

/// Makes the NOTCH window (only — the pet actively follows the user
/// instead, see `desktop_follow.rs`) visible on every virtual desktop, via
/// KWin's scripting API (`window.onAllDesktops = true`). Full story on why
/// this needs KWin scripting rather than `xdotool`/`wmctrl` is in
/// `kwin.rs`'s doc comment. Runs once, shortly after startup so the
/// window is mapped before KWin's `workspace.windowList()` is asked to
/// find it.
pub fn spawn_apply() {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await;
        let script = r#"
var wins = workspace.windowList();
for (var i = 0; i < wins.length; i++) {
    var w = wins[i];
    if (w.caption === "TuxBuddy Notch") {
        w.onAllDesktops = true;
    }
}
"#;
        kwin::run_script(script, "tuxbuddy-sticky");
    });
}
