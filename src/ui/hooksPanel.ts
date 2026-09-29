import { previewHookDiff, installHooks, uninstallHooks } from "../ipc/tauriBridge";

/**
 * Settings panel that drives the hook-installer commands already sitting
 * unused in the backend (PRD §4.4 #2 / §8 step 7): preview the merged
 * ~/.claude/settings.json, let the user confirm, then write it (the
 * backend takes its own backup before writing). Opened by clicking the
 * mascot (PRD §2.2 "Klik → menu") — the full menu's other items
 * (notification history, pause/diemin, info) are still undecided/future
 * (PRD §4.4 #6), so this is scoped to just the install/uninstall flow.
 */
export interface HooksPanel {
  toggle(): void;
  hide(): void;
}

export interface HooksPanelOptions {
  /** Called right when the menu becomes visible (not on every re-render
   * within it) — used to pause the roaming loop, see `main.ts`. */
  onOpen?: () => void;
  /** Called whenever the menu closes, from any path (Tutup, OK, Batal). */
  onClose?: () => void;
}

export function initHooksPanel(opts: HooksPanelOptions = {}): HooksPanel {
  const root = document.createElement("div");
  root.className = "hooks-panel";
  root.hidden = true;
  document.body.appendChild(root);

  function hide() {
    const wasOpen = !root.hidden;
    root.hidden = true;
    root.replaceChildren();
    if (wasOpen) opts.onClose?.();
  }

  function showStatus(message: string, isError = false) {
    root.replaceChildren();
    const title = document.createElement("div");
    title.className = "hooks-panel__title" + (isError ? " hooks-panel__title--error" : "");
    title.textContent = message;
    root.appendChild(title);

    const ok = document.createElement("button");
    ok.className = "hooks-panel__btn";
    ok.textContent = "OK";
    ok.onclick = hide;
    root.appendChild(ok);
  }

  async function showPreview() {
    root.replaceChildren();
    const title = document.createElement("div");
    title.className = "hooks-panel__title";
    title.textContent = "Preview perubahan ~/.claude/settings.json";
    root.appendChild(title);

    let diff: string;
    try {
      diff = await previewHookDiff();
    } catch (e) {
      showStatus(`Gagal membaca settings.json: ${e}`, true);
      return;
    }

    const pre = document.createElement("pre");
    pre.className = "hooks-panel__diff";
    pre.textContent = diff;
    root.appendChild(pre);

    const row = document.createElement("div");
    row.className = "hooks-panel__row";

    const confirm = document.createElement("button");
    confirm.className = "hooks-panel__btn hooks-panel__btn--primary";
    confirm.textContent = "Tulis (backup otomatis)";
    confirm.onclick = async () => {
      try {
        await installHooks();
        showStatus("Hooks terpasang. File lama di-backup sebagai settings.json.bak-<timestamp>.");
      } catch (e) {
        showStatus(`Gagal menulis settings.json: ${e}`, true);
      }
    };
    row.appendChild(confirm);

    const cancel = document.createElement("button");
    cancel.className = "hooks-panel__btn hooks-panel__btn--ghost";
    cancel.textContent = "Batal";
    cancel.onclick = () => void showMain();
    row.appendChild(cancel);

    root.appendChild(row);
  }

  function showMain() {
    const wasHidden = root.hidden;
    root.replaceChildren();
    root.hidden = false;
    if (wasHidden) opts.onOpen?.();

    const title = document.createElement("div");
    title.className = "hooks-panel__title";
    title.textContent = "TuxBuddy";
    root.appendChild(title);

    const install = document.createElement("button");
    install.className = "hooks-panel__btn hooks-panel__btn--primary";
    install.textContent = "Install hooks ke Claude Code…";
    install.onclick = () => void showPreview();
    root.appendChild(install);

    const uninstall = document.createElement("button");
    uninstall.className = "hooks-panel__btn hooks-panel__btn--danger";
    uninstall.textContent = "Uninstall hooks";
    uninstall.onclick = async () => {
      try {
        await uninstallHooks();
        showStatus("Hooks TuxBuddy dihapus dari settings.json (entri tool lain tidak disentuh).");
      } catch (e) {
        showStatus(`Gagal uninstall: ${e}`, true);
      }
    };
    root.appendChild(uninstall);

    const close = document.createElement("button");
    close.className = "hooks-panel__btn hooks-panel__btn--ghost";
    close.textContent = "Tutup";
    close.onclick = hide;
    root.appendChild(close);
  }

  return {
    toggle() {
      if (root.hidden) showMain();
      else hide();
    },
    hide,
  };
}
