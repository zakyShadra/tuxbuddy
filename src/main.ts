import { TuxRenderer } from "./tux/renderTux";
import { STATE, type SessionState } from "./tux/stateConfig";
import {
  listenForHookEvents,
  eventNameToState,
  focusTerminal,
  pauseRoaming,
  resumeRoaming,
} from "./ipc/tauriBridge";
import { initDecisionPanel } from "./ui/decisionPanel";
import { initHooksPanel } from "./ui/hooksPanel";

const canvas = document.getElementById("stage") as HTMLCanvasElement;
const renderer = new TuxRenderer(canvas);
const decisionPanel = initDecisionPanel();
const hooksPanel = initHooksPanel({
  // Roaming loop lives in the backend (see roaming.rs) — pause it while
  // this menu is open so Tux doesn't wander off out from under it.
  onOpen: () => void pauseRoaming(),
  onClose: () => void resumeRoaming(),
});

// Click opens the dropdown menu (PRD §2.2: "Klik → menu"). Right-click is
// kept as an alias since it doesn't conflict with anything. Scope is
// currently just the install/uninstall-hooks flow (PRD §4.4 #6's full menu
// — notification history, pause/diemin, "powered by Claude" info — is
// still undecided and left for later).
canvas.addEventListener("click", () => {
  hooksPanel.toggle();
});
canvas.addEventListener("contextmenu", (e) => {
  e.preventDefault();
  hooksPanel.toggle();
});

function loop(now: number) {
  renderer.frame(now);
  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);

canvas.addEventListener("dblclick", () => {
  // best-effort jump-to-terminal on click; harmless no-op until a real
  // hook envelope's meta has been seen (see App.currentMeta below)
  if (currentMeta) void focusTerminal(currentMeta);
});

let currentMeta: Parameters<typeof focusTerminal>[0] | null = null;

// Dev-only manual state switcher (see index.html comment) — lets us see
// every animation state before real Claude Code hook wiring exists.
const devStates = document.getElementById("dev-states")!;
for (const name of Object.keys(STATE) as SessionState[]) {
  const btn = document.createElement("button");
  btn.textContent = name;
  btn.style.setProperty("--accent", STATE[name].color);
  btn.onclick = () => {
    renderer.setState(name);
    document.querySelectorAll(".dev-states button").forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
  };
  devStates.appendChild(btn);
}

// Dev-only: preview the fly-in -> land -> decision-panel sequence without
// a real blocking hook event.
const flyBtn = document.createElement("button");
flyBtn.textContent = "Test Fly-In";
flyBtn.onclick = () => {
  renderer.setState("Approval");
  renderer.playFlyIn(() =>
    decisionPanel.show({
      id: "dev-test",
      event_name: "PreToolUse",
      blocking: true,
      payload: { tool_name: "Bash" },
      meta: {},
    }),
  );
};
devStates.appendChild(flyBtn);

// Real hook events from the Rust backend (crates/hook-cli -> Unix socket ->
// socket::router -> this event) take over state once they start arriving.
listenForHookEvents((envelope) => {
  currentMeta = envelope.meta;
  const state = eventNameToState(envelope.event_name);
  if (state) renderer.setState(state);

  // `blocking` (not event_name) decides whether a decision UI is needed —
  // see decisionPanel.ts for why. PRD §2.1 requires flying in and landing
  // *before* the UI appears, never both at once — playFlyIn's callback
  // enforces that ordering.
  if (envelope.blocking) {
    renderer.playFlyIn(() => decisionPanel.show(envelope));
  } else {
    decisionPanel.hide();
  }
});
