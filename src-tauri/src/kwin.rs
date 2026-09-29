use std::process::Command;

/// Shared helper for the KWin-scripting-based window tricks this app
/// needs on this Wayland/KWin session (see `sticky.rs`'s doc comment for
/// the full story on why: `xdotool`/`wmctrl` can't read or change a
/// window's virtual-desktop membership here, but KWin's own scripting API
/// over D-Bus can). Writes `js` to a temp file, loads it as a KWin script
/// under a unique plugin name, and runs it once. Returns `false` (and
/// does nothing further) if this isn't a KWin session / `qdbus6` isn't
/// available — every caller treats that as a silent no-op, not an error,
/// since this is a best-effort convenience on top of the platform, not a
/// hard requirement.
pub fn run_script(js: &str, plugin_prefix: &str) -> bool {
    // Unique plugin name per call — `tauri dev`'s file-watcher can restart
    // this process many times in one session, and repeated calls (e.g.
    // `desktop_follow`'s poll loop) need a fresh name each time too.
    let unique = format!(
        "{plugin_prefix}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_micros()).unwrap_or(0)
    );
    let path = std::env::temp_dir().join(format!("{unique}.js"));
    if std::fs::write(&path, js).is_err() {
        return false;
    }

    let load = Command::new("qdbus6")
        .args(["org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.loadScript", &path.to_string_lossy(), &unique])
        .output();
    let Ok(load) = load else {
        return false;
    };
    if !load.status.success() {
        return false;
    }

    let _ = Command::new("qdbus6")
        .args(["org.kde.KWin", "/Scripting", "org.kde.kwin.Scripting.start"])
        .status();
    // Deliberately left loaded rather than unloaded — see sticky.rs's
    // original reasoning (racing an immediate unload against async
    // execution felt riskier than a few stray tiny scripts staying
    // registered under unique names).
    true
}

/// Current virtual desktop index (1-based, matches KWin's own numbering)
/// — unlike `xdotool get_desktop`, this reads real Wayland-native KWin
/// state and was confirmed correct live (`xdotool`/`wmctrl` always
/// reported a stale "1 desktop total" on this session; this did not).
pub fn current_desktop() -> Option<String> {
    Command::new("qdbus6")
        .args(["org.kde.KWin", "/KWin", "org.kde.KWin.currentDesktop"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}
