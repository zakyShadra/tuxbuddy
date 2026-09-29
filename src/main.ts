import { PetRenderer } from "./tux/renderPet";
import {
  listenForHookEvents,
  listenForRoamMotion,
  listenForEntrance,
  listenForTap,
  focusTerminal,
  pauseRoaming,
  resumeRoaming,
  nudgePet,
} from "./ipc/tauriBridge";
import { initPetMenu } from "./ui/petMenu";

// PRD session-3 two-window split: this is the roaming "pet" window —
// ambient Idle/Walk/Run/Stop/Fly only, no state badges/glow/decision UI
// (that all moved to the notch window, see notch.ts). This window's
// on-screen position is driven entirely by the backend (`roaming.rs`)
// unless manual WASD control is on (see below).

const canvas = document.getElementById("stage") as HTMLCanvasElement;
const renderer = new PetRenderer(canvas);

// Roaming is paused whenever ANY of these are true, resumed only when
// none are — the menu, freeze toggle, and manual control all want the
// backend loop to leave the window alone, for different reasons.
let menuOpen = false;
let frozen = false;
let manualActive = false;
let entranceActive = false;
function syncRoaming() {
  if (menuOpen || frozen || manualActive) void pauseRoaming();
  else void resumeRoaming();
}

const NUDGE_STEP = 12; // physical px per WASD keypress
function onManualKeydown(e: KeyboardEvent) {
  const deltas: Record<string, [number, number]> = {
    w: [0, -NUDGE_STEP],
    a: [-NUDGE_STEP, 0],
    s: [0, NUDGE_STEP],
    d: [NUDGE_STEP, 0],
    ArrowUp: [0, -NUDGE_STEP],
    ArrowLeft: [-NUDGE_STEP, 0],
    ArrowDown: [0, NUDGE_STEP],
    ArrowRight: [NUDGE_STEP, 0],
  };
  const delta = deltas[e.key];
  if (!delta) return;
  e.preventDefault();
  void nudgePet(delta[0], delta[1]);
  renderer.setMoving(true, delta[0] < 0 ? -1 : 1);
}

const petMenu = initPetMenu({
  onOpen: () => {
    menuOpen = true;
    syncRoaming();
  },
  onClose: () => {
    menuOpen = false;
    syncRoaming();
  },
  freeze: {
    isEnabled: () => frozen,
    toggle: () => {
      frozen = !frozen;
      syncRoaming();
    },
  },
  manualControl: {
    isEnabled: () => manualActive,
    toggle: () => {
      manualActive = !manualActive;
      syncRoaming();
      if (manualActive) {
        document.addEventListener("keydown", onManualKeydown);
      } else {
        document.removeEventListener("keydown", onManualKeydown);
        renderer.setMoving(false, 1);
      }
    },
  },
});

// Click opens the dropdown menu (PRD §2.2: "Klik → menu"). Right-click is
// kept as an alias since it doesn't conflict with anything.
canvas.addEventListener("click", () => {
  petMenu.toggle();
});
canvas.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  petMenu.toggle();
});

function loop(now: number) {
  renderer.frame(now);
  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);

canvas.addEventListener("dblclick", () => {
  if (currentMeta) void focusTerminal(currentMeta);
});

let currentMeta: Parameters<typeof focusTerminal>[0] | null = null;

// roaming.rs emits this whenever movement starts/stops/reverses — see
// `renderPet.ts`'s setMoving for the walk<->idle<->stop-once handling.
// Ignored while manual control or an entrance sequence is driving the
// sprite directly — roaming.rs is paused during both, but still keeps
// emitting `moving:false` on every tick, which would otherwise stomp the
// entrance/manual pose back to idle mid-animation.
listenForRoamMotion(({ moving, dir }) => {
  if (manualActive || entranceActive) return;
  renderer.setMoving(moving, dir === -1 ? -1 : 1);
});

// desktop_follow.rs emits this after moving both windows onto the
// desktop the user just switched to — see `renderPet.ts`'s playEntrance.
listenForEntrance(() => {
  entranceActive = true;
  renderer.playEntrance(() => {
    entranceActive = false;
    void resumeRoaming();
  });
});

// doomscroll.rs emits this when a doomscroll-prone site looks open — see
// `renderPet.ts`'s playTap (placeholder gesture, real clip TBD).
listenForTap(() => {
  renderer.playTap();
});

// Still listen for hook events here (not just in notch.ts) purely to
// capture `meta` for double-click terminal-focus, and to trigger the
// fly-in sprite when a blocking envelope arrives (dormant while
// `is_blocking()` stays disabled backend-side, see PRD §4.1b #10 — kept
// wired for whenever that's revisited). The decision UI itself now lives
// in the notch window, not here.
listenForHookEvents((envelope) => {
  currentMeta = envelope.meta;
  if (envelope.blocking) {
    renderer.playFlyIn(() => {});
  }
});
