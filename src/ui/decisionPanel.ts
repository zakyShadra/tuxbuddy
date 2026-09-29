import { respondToEvent } from "../ipc/tauriBridge";
import type { HookEnvelope } from "../ipc/tauriBridge";

/**
 * The Allow/Always Allow/Deny + question-answer overlay. This is the
 * mascot's core function (PRD §2.2/§4.4 #1) — without it the app can react
 * to hook events visually but can never actually answer Claude Code back.
 *
 * Driven by `envelope.blocking`, not by `event_name`: the hook event schema
 * itself is still unconfirmed against a real Claude Code session (see
 * protocol::is_blocking doc comment), but `blocking === true` always means
 * a real pending id is waiting on the other end of the socket for a
 * decision, so that flag alone is enough to know a panel must be shown.
 */
export interface DecisionPanel {
  show(envelope: HookEnvelope): void;
  hide(): void;
}

export function initDecisionPanel(): DecisionPanel {
  const root = document.createElement("div");
  root.className = "decision-panel";
  root.hidden = true;
  document.body.appendChild(root);

  function hide() {
    root.hidden = true;
    root.replaceChildren();
  }

  function show(envelope: HookEnvelope) {
    root.replaceChildren();
    root.hidden = false;

    const title = document.createElement("div");
    title.className = "decision-panel__title";
    title.textContent = describeEnvelope(envelope);
    root.appendChild(title);

    if (isQuestionLike(envelope)) {
      buildQuestionUI(root, envelope, hide);
    } else {
      buildApprovalUI(root, envelope, hide);
    }
  }

  return { show, hide };
}

function buildApprovalUI(root: HTMLElement, envelope: HookEnvelope, hide: () => void) {
  const row = document.createElement("div");
  row.className = "decision-panel__row";

  // PROVISIONAL decision payload shapes — Claude Code's real hook response
  // schema is not yet confirmed (PRD §4.4 #12 / §8 step 8). These mirror
  // the two shapes named in protocol::DecisionMessage's doc comment.
  const buttons: Array<{ label: string; cls: string; decision: unknown }> = [
    { label: "Allow", cls: "decision-panel__btn--allow", decision: { behavior: "allow" } },
    {
      label: "Always Allow",
      cls: "decision-panel__btn--always",
      decision: { behavior: "allow", updatedPermissions: "always" },
    },
    { label: "Deny", cls: "decision-panel__btn--deny", decision: { behavior: "deny" } },
  ];

  for (const { label, cls, decision } of buttons) {
    const btn = document.createElement("button");
    btn.className = `decision-panel__btn ${cls}`;
    btn.textContent = label;
    btn.onclick = () => {
      void respondToEvent(envelope.id, decision);
      hide();
    };
    row.appendChild(btn);
  }
  root.appendChild(row);
}

function buildQuestionUI(root: HTMLElement, envelope: HookEnvelope, hide: () => void) {
  const input = document.createElement("input");
  input.type = "text";
  input.className = "decision-panel__input";
  input.placeholder = "Jawaban...";
  root.appendChild(input);

  const submit = document.createElement("button");
  submit.className = "decision-panel__btn decision-panel__btn--allow";
  submit.textContent = "Kirim";
  const send = () => {
    // PROVISIONAL — see buildApprovalUI comment.
    void respondToEvent(envelope.id, { answer: input.value });
    hide();
  };
  submit.onclick = send;
  input.onkeydown = (e) => {
    if (e.key === "Enter") send();
  };
  root.appendChild(submit);

  // Focus after the panel is actually in the layout.
  requestAnimationFrame(() => input.focus());
}

function isQuestionLike(envelope: HookEnvelope): boolean {
  const payload = envelope.payload as Record<string, unknown>;
  return payload.tool_name === "AskUserQuestion" || typeof payload.question === "string";
}

function describeEnvelope(envelope: HookEnvelope): string {
  const payload = envelope.payload as Record<string, unknown>;
  const question = typeof payload.question === "string" ? payload.question : undefined;
  const message = typeof payload.message === "string" ? payload.message : undefined;
  const toolName = typeof payload.tool_name === "string" ? payload.tool_name : undefined;
  return question ?? message ?? (toolName ? `Allow ${toolName}?` : envelope.event_name);
}
