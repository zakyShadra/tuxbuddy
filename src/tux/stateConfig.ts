// Mirrors `SessionState` in src-tauri/src/session.rs — keep the variant
// names identical (PascalCase) so tauriBridge.ts needs no translation table.
export type SessionState =
  | "Idle"
  | "Working"
  | "Thinking"
  | "Searching"
  | "Approval"
  | "Question"
  | "Error"
  | "Finished"
  | "RateLimit";

export type Badge = "dots" | "bang" | "question" | "dot" | null;

export interface StateCfg {
  glow: string;
  glowOp: number;
  badge: Badge;
  bounce: boolean;
  tilt: number;
  color: string;
  wobble?: boolean;
  shake?: boolean;
  burst?: boolean;
  tired?: boolean;
}

export const STATE: Record<SessionState, StateCfg> = {
  Idle: { glow: "#5b6472", glowOp: 0.16, badge: null, bounce: false, tilt: 0, color: "#e5e7eb" },
  Working: { glow: "#3b82f6", glowOp: 0.55, badge: "dots", bounce: true, tilt: 0, color: "#3b82f6" },
  Thinking: { glow: "#8b5cf6", glowOp: 0.5, badge: "dots", bounce: false, tilt: -0.06, color: "#8b5cf6" },
  Searching: { glow: "#6366f1", glowOp: 0.55, badge: "dots", bounce: false, tilt: 0, color: "#6366f1", wobble: true },
  Approval: { glow: "#f59e0b", glowOp: 0.6, badge: "bang", bounce: true, tilt: 0, color: "#f59e0b" },
  Question: { glow: "#22d3ee", glowOp: 0.55, badge: "question", bounce: false, tilt: 0.12, color: "#22d3ee" },
  Error: { glow: "#f43f5e", glowOp: 0.55, badge: "dot", bounce: false, tilt: 0, color: "#f43f5e", shake: true },
  Finished: { glow: "#22c55e", glowOp: 0.5, badge: "dot", bounce: false, tilt: 0, color: "#22c55e", burst: true },
  RateLimit: { glow: "#fb923c", glowOp: 0.4, badge: "dot", bounce: false, tilt: 0, color: "#fb923c", tired: true },
};
