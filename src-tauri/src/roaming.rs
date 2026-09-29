use std::process::Command;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use tokio::sync::mpsc;

/// One roaming tick, in milliseconds — ~30Hz is smooth enough for a
/// slow-walking pet and cheap on set_position() calls.
const TICK_MS: u64 = 33;
/// Idle roam speed along the bottom edge, physical px/s.
const ROAM_SPEED: f64 = 46.0;
/// Gap kept between Tux's feet and the screen's bottom edge, physical px.
const BOTTOM_MARGIN: f64 = 18.0;
/// How long the fly-to-user glide takes — matches `playFlyIn`'s 60-frame
/// clip at 24fps (src/tux/renderTux.ts, ~2.5s) so the window arrives at
/// its target right as the landing frame does.
const FLY_MS: f64 = 2500.0;
/// Title substrings that identify our own windows — used to ignore them
/// when looking for "the window the user is actually working in" so Tux
/// doesn't try to fly to itself after being clicked (see `active_window_target`).
const OWN_WINDOW_TITLES: [&str; 2] = ["TuxBuddy", "TuxBuddy Notch"];

pub enum RoamCommand {
    Pause,
    Resume,
    /// Fly to wherever the user is actually working (PRD session-3
    /// decision: "nyamperin usernya", not just the center of whichever
    /// monitor the pet happens to be on).
    FlyToUser,
}

/// Cheap `Clone` handle for sending commands into the roaming task —
/// managed as Tauri state (see `lib.rs`) so `commands.rs` and
/// `socket/router.rs` can both reach it.
#[derive(Clone)]
pub struct RoamHandle {
    tx: mpsc::UnboundedSender<RoamCommand>,
}

impl RoamHandle {
    pub fn pause(&self) {
        let _ = self.tx.send(RoamCommand::Pause);
    }
    pub fn resume(&self) {
        let _ = self.tx.send(RoamCommand::Resume);
    }
    pub fn fly_to_user(&self) {
        let _ = self.tx.send(RoamCommand::FlyToUser);
    }
}

struct MonitorSpan {
    x0: i32,
    x1: i32,
    top: i32,
    bottom: i32,
}

/// All monitors' horizontal spans, sorted left-to-right, in physical
/// pixels. Treating them as one contiguous walkable strip (rather than
/// modeling gaps/misalignment precisely) is a deliberate simplification —
/// good enough to let Tux "cross into" an adjacent monitor per PRD §2.1,
/// without needing exact multi-monitor topology.
fn monitor_spans(window: &WebviewWindow) -> Vec<MonitorSpan> {
    let mut spans: Vec<MonitorSpan> = window
        .available_monitors()
        .ok()
        .into_iter()
        .flatten()
        .map(|m| {
            let pos = m.position();
            let size = m.size();
            MonitorSpan {
                x0: pos.x,
                x1: pos.x + size.width as i32,
                top: pos.y,
                bottom: pos.y + size.height as i32,
            }
        })
        .collect();
    spans.sort_by_key(|s| s.x0);
    spans
}

fn span_at(spans: &[MonitorSpan], x: f64) -> Option<&MonitorSpan> {
    spans
        .iter()
        .find(|s| (s.x0 as f64) <= x && x < s.x1 as f64)
        .or_else(|| spans.last())
}

fn ease_out_cubic(t: f64) -> f64 {
    let p = 1.0 - t;
    1.0 - p * p * p
}

/// Best-effort "where is the user actually working" — X11 only (`xdotool`,
/// same constraint as `terminal_focus.rs`). Returns the active window's
/// center in physical px, or `None` if `xdotool` is unavailable, nothing
/// is focused, or the focused window is one of TuxBuddy's own (so Tux
/// never "flies to itself" right after being clicked).
fn active_window_target() -> Option<(f64, f64)> {
    let name_out = Command::new("xdotool").args(["getactivewindow", "getwindowname"]).output().ok()?;
    if !name_out.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&name_out.stdout);
    if OWN_WINDOW_TITLES.iter().any(|t| name.contains(t)) {
        return None;
    }

    let geo_out = Command::new("xdotool")
        .args(["getactivewindow", "getwindowgeometry", "--shell"])
        .output()
        .ok()?;
    if !geo_out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&geo_out.stdout);
    let mut x: Option<f64> = None;
    let mut y: Option<f64> = None;
    let mut w: Option<f64> = None;
    let mut h: Option<f64> = None;
    for line in text.lines() {
        let (key, val) = line.split_once('=')?;
        let parsed = val.parse::<f64>().ok();
        match key {
            "X" => x = parsed,
            "Y" => y = parsed,
            "WIDTH" => w = parsed,
            "HEIGHT" => h = parsed,
            _ => {}
        }
    }
    Some((x? + w? / 2.0, y? + h? / 2.0))
}

enum Mode {
    Roaming,
    Paused,
    Flying {
        from: (f64, f64),
        to: (f64, f64),
        start: Instant,
    },
    Centered,
}

impl Mode {
    fn is_moving(&self) -> bool {
        matches!(self, Mode::Roaming)
    }
}

/// Spawns the background loop that walks Tux's window along the bottom
/// edge of the monitor strip (PRD §2.1 "Idle: Tux jalan santai... bisa
/// nyebrang ke monitor lain") and, on command, glides it to wherever the
/// user is actively working — physically moving the window, not just the
/// sprite animation (PRD §4.4 #2, refined in session 3 to target the
/// focused window instead of a monitor's bare center).
///
/// Also emits `tuxbuddy://roam-motion` (`{ moving, dir }`) on the "main"
/// window whenever movement starts/stops or reverses direction, so the
/// frontend can switch between the idle-standing and walk-cycle sequences
/// and mirror the sprite to face the direction of travel — see
/// `src/main.ts` and `src/tux/renderTux.ts`.
///
/// Runs continuously across every non-attention `SessionState` (not just
/// `Idle`) — a desktop pet that only moves while literally idle would
/// spend most of a session frozen mid-screen. It only stops for
/// `RoamCommand::Pause` (attention UI open, hooks menu open) or
/// `RoamCommand::FlyToUser`.
pub fn spawn(app: AppHandle) -> RoamHandle {
    let (tx, mut rx) = mpsc::unbounded_channel::<RoamCommand>();

    tauri::async_runtime::spawn(async move {
        let mut mode = Mode::Roaming;
        let mut pos_x = app
            .get_webview_window("main")
            .and_then(|w| w.outer_position().ok())
            .map(|p| p.x as f64)
            .unwrap_or(0.0);
        let mut vx: f64 = ROAM_SPEED;
        let mut interval = tokio::time::interval(Duration::from_millis(TICK_MS));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let Some(window) = app.get_webview_window("main") else { continue };
                    match &mode {
                        Mode::Roaming => step_roam(&window, &mut pos_x, &mut vx),
                        Mode::Flying { from, to, start } => {
                            let t = start.elapsed().as_millis() as f64 / FLY_MS;
                            if t >= 1.0 {
                                let _ = window.set_position(PhysicalPosition::new(to.0 as i32, to.1 as i32));
                                pos_x = to.0;
                                mode = Mode::Centered;
                            } else {
                                let e = ease_out_cubic(t);
                                let x = from.0 + (to.0 - from.0) * e;
                                let y = from.1 + (to.1 - from.1) * e;
                                let _ = window.set_position(PhysicalPosition::new(x as i32, y as i32));
                            }
                        }
                        Mode::Paused | Mode::Centered => {}
                    }
                    emit_motion(&app, &mode, vx);
                }
                cmd = rx.recv() => {
                    let Some(cmd) = cmd else { break };
                    let Some(window) = app.get_webview_window("main") else { continue };
                    match cmd {
                        RoamCommand::Pause => mode = Mode::Paused,
                        RoamCommand::Resume => {
                            // Resync from wherever the window actually is
                            // (e.g. still centered from a FlyToUser) so
                            // roaming continues smoothly instead of
                            // snapping back to the last roam position.
                            if let Ok(p) = window.outer_position() {
                                pos_x = p.x as f64;
                            }
                            mode = Mode::Roaming;
                        }
                        RoamCommand::FlyToUser => {
                            if let (Ok(cur), Ok(size)) = (window.outer_position(), window.outer_size()) {
                                let target = active_window_target().or_else(|| {
                                    // Fall back to "center of whichever monitor
                                    // the pet is currently on" when there's no
                                    // usable active-window signal (no xdotool,
                                    // nothing focused, or focus is on TuxBuddy
                                    // itself).
                                    let spans = monitor_spans(&window);
                                    let center_x = cur.x as f64 + size.width as f64 / 2.0;
                                    span_at(&spans, center_x).map(|span| {
                                        (
                                            span.x0 as f64 + (span.x1 - span.x0) as f64 / 2.0,
                                            span.top as f64 + (span.bottom - span.top) as f64 / 2.0,
                                        )
                                    })
                                });
                                if let Some((tx_, ty_)) = target {
                                    let spans = monitor_spans(&window);
                                    let span = span_at(&spans, tx_);
                                    // Clamp so the pet window's own bounds stay
                                    // fully on-screen even when the target is
                                    // near a monitor's edge.
                                    let (min_x, max_x, min_y, max_y) = match span {
                                        Some(s) => (
                                            s.x0 as f64,
                                            s.x1 as f64 - size.width as f64,
                                            s.top as f64,
                                            s.bottom as f64 - size.height as f64,
                                        ),
                                        None => (tx_, tx_, ty_, ty_),
                                    };
                                    let to_x = (tx_ - size.width as f64 / 2.0).clamp(min_x, max_x.max(min_x));
                                    let to_y = (ty_ - size.height as f64 / 2.0).clamp(min_y, max_y.max(min_y));
                                    mode = Mode::Flying {
                                        from: (cur.x as f64, cur.y as f64),
                                        to: (to_x, to_y),
                                        start: Instant::now(),
                                    };
                                }
                            }
                        }
                    }
                    emit_motion(&app, &mode, vx);
                }
            }
        }
    });

    RoamHandle { tx }
}

#[derive(Clone, serde::Serialize)]
struct RoamMotion {
    moving: bool,
    dir: i32,
}

/// Emitted every tick (not just on change) — a change-only version raced
/// with the pet window's JS taking longer than one 33ms tick to attach its
/// listener, so the very first `moving:true` at startup was silently
/// missed and the sprite never left the idle pose despite the window
/// visibly roaming. Emitting continuously means any listener that attaches
/// late still gets the current state within one tick, at the cost of a
/// steady ~30Hz stream of a two-field JSON payload — cheap enough locally.
fn emit_motion(app: &AppHandle, mode: &Mode, vx: f64) {
    let moving = mode.is_moving();
    let dir = if vx >= 0.0 { 1 } else { -1 };
    let _ = app.emit_to("main", "tuxbuddy://roam-motion", RoamMotion { moving, dir });
}

fn step_roam(window: &WebviewWindow, pos_x: &mut f64, vx: &mut f64) {
    let spans = monitor_spans(window);
    if spans.is_empty() {
        return;
    }
    let Ok(size) = window.outer_size() else { return };
    let win_w = size.width as f64;
    let win_h = size.height as f64;

    let min_x = spans.first().unwrap().x0 as f64;
    let max_x = spans.last().unwrap().x1 as f64;

    *pos_x += *vx * (TICK_MS as f64 / 1000.0);
    if *pos_x <= min_x {
        *pos_x = min_x;
        *vx = vx.abs();
    } else if *pos_x + win_w >= max_x {
        *pos_x = max_x - win_w;
        *vx = -vx.abs();
    }

    // Pick whichever monitor the window's horizontal center currently sits
    // over, so its bottom edge (not some other monitor's) drives Y —
    // that's what makes crossing into an adjacent monitor with a different
    // resolution/position look right instead of floating or sinking.
    let center_x = *pos_x + win_w / 2.0;
    let span = span_at(&spans, center_x).unwrap();
    let y = span.bottom as f64 - win_h - BOTTOM_MARGIN;

    let _ = window.set_position(PhysicalPosition::new(*pos_x as i32, y as i32));
}
