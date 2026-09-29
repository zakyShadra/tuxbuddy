/**
 * The roaming pet's tap menu (PRD §2.2 "Klik → menu") — kept deliberately
 * small: freeze the roaming loop, and toggle opt-in WASD manual control
 * (§4.4 #5). Everything else (hooks install/uninstall, notification
 * history, "powered by Claude" info) moved to the notch's hover-expand
 * flyout (`notch.ts`) per live session-3 feedback — this panel used to
 * hold the hooks install/uninstall flow too (see git history / PRD if
 * that's ever needed again), but the pet is meant to be a simple
 * "stop/control" surface, not a settings panel.
 */
export interface PetMenu {
  toggle(): void;
  hide(): void;
}

export interface PetMenuOptions {
  /** Called right when the menu becomes visible — used to pause the
   * roaming loop, see `main.ts`. */
  onOpen?: () => void;
  /** Called whenever the menu closes, from any path. */
  onClose?: () => void;
  freeze: { isEnabled: () => boolean; toggle: () => void };
  manualControl: { isEnabled: () => boolean; toggle: () => void };
}

export function initPetMenu(opts: PetMenuOptions): PetMenu {
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

  function toggleBtn(getLabel: () => string, onToggle: () => void): HTMLButtonElement {
    const btn = document.createElement("button");
    btn.className = "hooks-panel__btn";
    const refresh = () => (btn.textContent = getLabel());
    refresh();
    btn.onclick = () => {
      onToggle();
      refresh();
    };
    return btn;
  }

  function show() {
    const wasHidden = root.hidden;
    root.replaceChildren();
    root.hidden = false;
    if (wasHidden) opts.onOpen?.();

    const title = document.createElement("div");
    title.className = "hooks-panel__title";
    title.textContent = "TuxBuddy";
    root.appendChild(title);

    root.appendChild(
      toggleBtn(
        () => (opts.freeze.isEnabled() ? "Diemin: AKTIF — tap utk lanjut jalan" : "Diemin Tux (stop roaming)"),
        opts.freeze.toggle,
      ),
    );

    root.appendChild(
      toggleBtn(
        () =>
          opts.manualControl.isEnabled()
            ? "Kontrol manual (WASD): AKTIF — tap utk matikan"
            : "Kontrol manual (WASD): mati — tap utk aktifkan",
        opts.manualControl.toggle,
      ),
    );

    const close = document.createElement("button");
    close.className = "hooks-panel__btn hooks-panel__btn--ghost";
    close.textContent = "Tutup";
    close.onclick = hide;
    root.appendChild(close);
  }

  return {
    toggle() {
      if (root.hidden) show();
      else hide();
    },
    hide,
  };
}
