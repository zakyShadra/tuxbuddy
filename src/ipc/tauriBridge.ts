import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SessionState } from "../tux/stateConfig";

// Mirrors `protocol::HookEnvelope` (crates/protocol/src/lib.rs) as it
// arrives over the `tuxbuddy://hook-event` Tauri event.
export interface HookEnvelope {
  id: string;
  event_name: string;
  blocking: boolean;
  payload: Record<string, unknown>;
  meta: {
    term_program?: string | null;
    term_session_id?: string | null;
    cwd?: string | null;
    pid?: number | null;
  };
}

/**
 * Maps a raw Claude Code hook event name to the mascot's SessionState.
 *
 * PROVISIONAL — the exact event names/fields have not yet been confirmed
 * against a real Claude Code session (plan M0 "schema discovery" step).
 * Deliberately kept as one small function so a correction here doesn't
 * ripple through the renderer or the socket layer.
 */
export function eventNameToState(eventName: string): SessionState | null {
  switch (eventName) {
    case "SessionStart":
      return "Idle";
    case "UserPromptSubmit":
    case "PreToolUse":
    case "PostToolUse":
      return "Working";
    case "PermissionRequest":
      return "Approval";
    case "Notification":
      return "Question";
    case "PostToolUseFailure":
    case "StopFailure":
      return "Error";
    case "Stop":
    case "SessionEnd":
      return "Finished";
    case "SubagentStart":
      return "Thinking";
    case "SubagentStop":
      return "Finished";
    default:
      return null;
  }
}

export function listenForHookEvents(cb: (envelope: HookEnvelope) => void): Promise<() => void> {
  return listen<HookEnvelope>("tuxbuddy://hook-event", (event) => cb(event.payload));
}

export function respondToEvent(id: string, decision: unknown): Promise<void> {
  return invoke("respond_to_event", { id, decision });
}

export function getSessionState(): Promise<SessionState> {
  return invoke("get_session_state");
}

export function focusTerminal(meta: HookEnvelope["meta"]): Promise<void> {
  return invoke("focus_terminal", { meta });
}

export function previewHookDiff(): Promise<string> {
  return invoke("preview_hook_diff");
}

export function installHooks(): Promise<void> {
  return invoke("install_hooks");
}

export function uninstallHooks(): Promise<void> {
  return invoke("uninstall_hooks");
}

// PRD §4.4 #1/#4: pause the backend roaming loop while the hooks/menu
// panel is open, so Tux doesn't wander off out from under it.
export function pauseRoaming(): Promise<void> {
  return invoke("pause_roaming");
}

export function resumeRoaming(): Promise<void> {
  return invoke("resume_roaming");
}
