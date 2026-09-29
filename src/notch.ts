import { TuxRenderer } from "./tux/renderTux";
import { eventNameToState, listenForHookEvents } from "./ipc/tauriBridge";
import { mediaList, mediaPlayPause, mediaNext, mediaPrevious, type PlayerInfo } from "./ipc/tauriBridge";
import { initDecisionPanel } from "./ui/decisionPanel";

// PRD session-3 two-window split, redesigned live several times into a
// 3-state Dynamic-Island-style pill:
//   1. collapsed — idle, no hover: tiny circular avatar (current
//      SessionState animation)
//   2. row       — hover: expands sideways into a slim bar with 3 tabs
//      (Notif / Musik / Info)
//   3. panel     — a tab tapped: expands further to show that tab's
//      full content
// Hooks install/uninstall is no longer a tab — it's auto-installed on
// every app startup now (see src-tauri/src/lib.rs's `.setup()`), per
// explicit user direction ("buat jadi default aja").

const canvas = document.getElementById("stage") as HTMLCanvasElement;
const pill = document.getElementById("notch-pill")!;
const panel = document.getElementById("notch-panel")!;
const renderer = new TuxRenderer(canvas, { avatarMode: true });
const decisionPanel = initDecisionPanel();

function loop(now: number) {
  renderer.frame(now);
  requestAnimationFrame(loop);
}
requestAnimationFrame(loop);

listenForHookEvents((envelope) => {
  const state = eventNameToState(envelope.event_name);
  if (state) renderer.setState(state);

  // Dormant while `is_blocking()` stays disabled backend-side (PRD
  // §4.1b #10) — kept wired for whenever that's revisited.
  if (envelope.blocking) {
    decisionPanel.show(envelope);
  } else {
    decisionPanel.hide();
  }
});

// --- Pill state machine -----------------------------------------------

type Tab = "notif" | "music" | "info";
let activeTab: Tab | null = null;
let musicPollHandle: ReturnType<typeof setInterval> | null = null;

function collapse() {
  pill.classList.remove("row", "panel");
  activeTab = null;
  document.querySelectorAll<HTMLElement>(".notch-tab").forEach((t) => t.classList.remove("active"));
  stopMusicPoll();
}

function openTab(tab: Tab) {
  activeTab = tab;
  pill.classList.add("row", "panel");
  document.querySelectorAll<HTMLElement>(".notch-tab").forEach((t) => t.classList.toggle("active", t.dataset.tab === tab));
  render();
  if (tab === "music") startMusicPoll();
  else stopMusicPoll();
}

// Entering expands instantly; leaving waits 2s before collapsing (PRD
// session-3: "kasih jeda dong antara cursor keluar... datengnya tetep
// jangan kasih jeda") — so briefly overshooting the pill's edge while
// moving toward a tab/button doesn't instantly snap it shut.
const COLLAPSE_DELAY_MS = 2000;
let collapseTimer: ReturnType<typeof setTimeout> | null = null;

function cancelPendingCollapse() {
  if (collapseTimer !== null) {
    clearTimeout(collapseTimer);
    collapseTimer = null;
  }
}

pill.addEventListener("mouseenter", () => {
  cancelPendingCollapse();
  if (!activeTab) pill.classList.add("row");
});
pill.addEventListener("mouseleave", () => {
  cancelPendingCollapse();
  collapseTimer = setTimeout(collapse, COLLAPSE_DELAY_MS);
});

document.querySelectorAll<HTMLElement>(".notch-tab").forEach((btn) => {
  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    const tab = btn.dataset.tab as Tab;
    if (activeTab === tab) {
      pill.classList.remove("panel");
      activeTab = null;
      btn.classList.remove("active");
      stopMusicPoll();
    } else {
      openTab(tab);
    }
  });
});

// --- Panel content -------------------------------------------------

function titleEl(text: string) {
  const d = document.createElement("div");
  d.className = "notch-panel__title";
  d.textContent = text;
  return d;
}

function render() {
  panel.replaceChildren();
  if (activeTab === "notif") renderNotif();
  else if (activeTab === "music") renderMusic();
  else if (activeTab === "info") renderInfo();
}

function renderNotif() {
  panel.appendChild(titleEl("Riwayat notifikasi"));
  const empty = document.createElement("div");
  empty.className = "notch-empty";
  empty.textContent = "Belum ada riwayat tersimpan — fitur ini masih placeholder.";
  panel.appendChild(empty);
}

// --- Info tab: user-editable note (PRD session-3: "isi info bisa diganti
// user", not a fixed app blurb) — stored in this window's localStorage,
// private to the notch webview. ---

const INFO_STORAGE_KEY = "tuxbuddy-notch-info";
const DEFAULT_INFO = "Catatan kamu di sini. Tap Edit buat ganti.";

function loadInfoText(): string {
  try {
    return localStorage.getItem(INFO_STORAGE_KEY) ?? DEFAULT_INFO;
  } catch {
    return DEFAULT_INFO;
  }
}

function saveInfoText(text: string) {
  try {
    localStorage.setItem(INFO_STORAGE_KEY, text);
  } catch {
    // Best-effort — private-window-style storage blocks are rare in a
    // Tauri webview but not impossible; the edit just won't persist.
  }
}

function renderInfo() {
  panel.appendChild(titleEl("Info"));
  const body = document.createElement("div");
  body.className = "notch-panel__body";
  body.textContent = loadInfoText();
  panel.appendChild(body);

  const editBtn = document.createElement("button");
  editBtn.className = "notch-edit-btn";
  editBtn.textContent = "Edit";
  editBtn.onclick = (e) => {
    e.stopPropagation();
    body.hidden = true;
    editBtn.hidden = true;

    const textarea = document.createElement("textarea");
    textarea.className = "notch-info-textarea";
    textarea.value = loadInfoText();

    const row = document.createElement("div");
    row.className = "notch-media-controls";
    const save = document.createElement("button");
    save.className = "notch-edit-btn";
    save.textContent = "Simpan";
    save.onclick = (ev) => {
      ev.stopPropagation();
      saveInfoText(textarea.value);
      textarea.remove();
      row.remove();
      body.textContent = textarea.value;
      body.hidden = false;
      editBtn.hidden = false;
    };
    const cancel = document.createElement("button");
    cancel.className = "notch-edit-btn";
    cancel.textContent = "Batal";
    cancel.onclick = (ev) => {
      ev.stopPropagation();
      textarea.remove();
      row.remove();
      body.hidden = false;
      editBtn.hidden = false;
    };
    row.appendChild(save);
    row.appendChild(cancel);
    panel.insertBefore(textarea, editBtn);
    panel.insertBefore(row, editBtn);
  };
  panel.appendChild(editBtn);
}

// --- Music tab: shows every detected MPRIS player as a scrollable list
// (PRD session-3: "biar bisa dipilih mau control yang mana lewat
// scrolling") with the selected one's full controls above it. Selection
// deliberately does NOT auto-follow whichever player happens to be
// "Playing" — an earlier version did, and it meant pausing your music
// silently flipped the notch over to a different player entirely
// (reported live as jarring, "mending tahan dulu aja jangan langsung
// dipindahin"). It only re-picks a default when the selected player
// disappears from the list (i.e. the app/tab actually closed). ---

let selectedPlayer: string | null = null;
let musicEmpty: HTMLElement | null = null;
let musicTrack: { root: HTMLElement; title: HTMLElement; artist: HTMLElement } | null = null;
let musicControls: { root: HTMLElement; prev: HTMLButtonElement; playPause: HTMLButtonElement; next: HTMLButtonElement } | null = null;
let musicListEl: HTMLElement | null = null;
const musicRows = new Map<string, { root: HTMLButtonElement; dot: HTMLElement; text: HTMLElement }>();

function renderMusic() {
  panel.appendChild(titleEl("Music Player"));

  musicEmpty = document.createElement("div");
  musicEmpty.className = "notch-empty";
  musicEmpty.textContent = "Tidak ada player aktif (Spotify/browser lewat MPRIS).";
  musicEmpty.hidden = true;
  panel.appendChild(musicEmpty);

  const trackRoot = document.createElement("div");
  trackRoot.className = "notch-track";
  const title = document.createElement("div");
  title.className = "notch-track__title";
  const artist = document.createElement("div");
  artist.className = "notch-track__artist";
  trackRoot.appendChild(title);
  trackRoot.appendChild(artist);
  panel.appendChild(trackRoot);
  musicTrack = { root: trackRoot, title, artist };

  const controlsRoot = document.createElement("div");
  controlsRoot.className = "notch-media-controls";
  const prev = document.createElement("button");
  prev.className = "notch-media-btn notch-media-btn--ghost";
  prev.textContent = "⏮";
  prev.onclick = async (e) => {
    e.stopPropagation();
    if (selectedPlayer) await mediaPrevious(selectedPlayer);
    void refreshMusic();
  };
  const playPause = document.createElement("button");
  playPause.className = "notch-media-btn";
  playPause.onclick = async (e) => {
    e.stopPropagation();
    if (selectedPlayer) await mediaPlayPause(selectedPlayer);
    void refreshMusic();
  };
  const next = document.createElement("button");
  next.className = "notch-media-btn notch-media-btn--ghost";
  next.textContent = "⏭";
  next.onclick = async (e) => {
    e.stopPropagation();
    if (selectedPlayer) await mediaNext(selectedPlayer);
    void refreshMusic();
  };
  controlsRoot.appendChild(prev);
  controlsRoot.appendChild(playPause);
  controlsRoot.appendChild(next);
  panel.appendChild(controlsRoot);
  musicControls = { root: controlsRoot, prev, playPause, next };

  musicListEl = document.createElement("div");
  musicListEl.className = "notch-player-list";
  panel.appendChild(musicListEl);
  musicRows.clear();

  void refreshMusic();
}

function selectPlayer(id: string) {
  selectedPlayer = id;
  void refreshMusic();
}

async function refreshMusic() {
  if (activeTab !== "music" || !musicEmpty || !musicTrack || !musicControls || !musicListEl) return;
  let players: PlayerInfo[];
  try {
    players = await mediaList();
  } catch {
    players = [];
  }

  const stillPresent = players.some((p) => p.id === selectedPlayer);
  if (!selectedPlayer || !stillPresent) {
    selectedPlayer = players.find((p) => p.playing)?.id ?? players[0]?.id ?? null;
  }

  musicEmpty.hidden = players.length > 0;
  musicTrack.root.hidden = players.length === 0;
  musicControls.root.hidden = players.length === 0;

  const selected = players.find((p) => p.id === selectedPlayer);
  if (selected) {
    musicTrack.title.textContent = selected.title || "(tanpa judul)";
    musicTrack.artist.textContent = selected.artist || "";
    musicControls.playPause.textContent = selected.playing ? "⏸" : "▶";
  }

  // Keyed diff against the existing rows — mutate in place, only
  // add/remove DOM nodes for players that actually appeared/disappeared,
  // same "don't replace what might be under the cursor" reasoning as the
  // single-player version had (see PRD §4.1e point 4).
  const seen = new Set<string>();
  for (const p of players) {
    seen.add(p.id);
    let row = musicRows.get(p.id);
    if (!row) {
      const rootBtn = document.createElement("button");
      rootBtn.className = "notch-player-row";
      const dot = document.createElement("span");
      dot.className = "notch-player-row__dot";
      const text = document.createElement("span");
      text.className = "notch-player-row__text";
      rootBtn.appendChild(dot);
      rootBtn.appendChild(text);
      rootBtn.onclick = (e) => {
        e.stopPropagation();
        selectPlayer(p.id);
      };
      musicListEl.appendChild(rootBtn);
      row = { root: rootBtn, dot, text };
      musicRows.set(p.id, row);
    }
    row.root.classList.toggle("selected", p.id === selectedPlayer);
    row.root.classList.toggle("playing", p.playing);
    row.text.textContent = `${p.id}: ${p.title || "(tanpa judul)"}`;
  }
  for (const [id, row] of musicRows) {
    if (!seen.has(id)) {
      row.root.remove();
      musicRows.delete(id);
    }
  }
}

function startMusicPoll() {
  stopMusicPoll();
  // renderMusic() (called just before this, from openTab) already does
  // the first refreshMusic() — this only needs to set up the recurring one.
  musicPollHandle = setInterval(() => void refreshMusic(), 2000);
}

function stopMusicPoll() {
  if (musicPollHandle !== null) {
    clearInterval(musicPollHandle);
    musicPollHandle = null;
  }
}
