import { Ease } from "../engine/easing";
import { Tween } from "../engine/tween";
import { spawnParticle, updateParticles, drawParticles } from "../engine/particles";
import { STATE, type SessionState } from "./stateConfig";

const SPRITE_H = 372;

// Source video is 24fps and every frame was kept (no subsampling), so
// playing back at 24fps reproduces the original motion speed exactly.
const FRAME_MS = 1000 / 24;

// Per-state animated frame sequences, extracted from the user's
// Gemini-generated tux.mp4 (Thinking/Working/Searching/Finished/Error/
// Question/Approval) and idle-run.mp4 (Idle, standing portion only — the
// running/profile part is reserved for a future roaming animation, see
// PRD §5), each background-removed with rembg/u2net (PRD §5). Frames are
// named f000.png.. under src/assets/states/<dir>/. RateLimit has no
// matching scene in either source video, so it reuses the Idle sequence
// (the glow/tired-particle treatment in stateConfig.ts carries the state
// instead of a dedicated pose).
const FRAME_DIR: Record<SessionState, string> = {
  Idle: "idle",
  Thinking: "thinking",
  Working: "working",
  Searching: "searching",
  Finished: "finished",
  Error: "error",
  Question: "question",
  Approval: "approval",
  RateLimit: "idle",
};

const FRAME_COUNT: Record<string, number> = {
  idle: 87,
  thinking: 28,
  working: 31,
  searching: 28,
  finished: 33,
  error: 28,
  question: 29,
  approval: 28,
};

// avatarMode (notch's 56px circle) face-crop calibration, per SessionState
// — a shared crop calibrated on Idle alone clipped badly on the ~5 states
// whose source scene is a wide landscape shot (character off to one side
// next to a prop, e.g. Working's laptop) rather than Idle/Thinking/
// Approval's portrait-centered framing. Values are (faceFracX, faceFracY):
// the point in the ORIGINAL frame (as a 0-1 fraction of its width/height)
// that gets centered in the circle — found by cropping each state's f000
// at a guessed region, looking at the result, and adjusting (see PRD
// §4.1f/session-3 for the one-off Python script used). `zoom` is shared
// since it mainly controls "how tight" the crop is, not position.
// Lowered twice now from the value verified via rectangular Python crops
// (2.2 -> 1.5 -> 1.1) — those crops didn't account for the circular CSS
// mask the notch actually applies (border-radius:50%), which clips the
// rectangle's corners and made the live result noticeably tighter than
// the verification images. 1.5 was still too tight for Working
// specifically — reported live ("laptop nya ngga kebawa"): its crop
// center is off to one side (faceFracX=0.775) specifically to include
// the laptop prop, and less zoom is needed to keep both the prop and the
// face in frame at once.
const AVATAR_ZOOM = 1.1;
const AVATAR_FACE_FRAC: Record<SessionState, [number, number]> = {
  Idle: [0.5, 0.225],
  Thinking: [0.48, 0.32],
  Working: [0.775, 0.325],
  Searching: [0.72, 0.32],
  Finished: [0.72, 0.28],
  Error: [0.7, 0.245],
  Question: [0.43, 0.28],
  Approval: [0.5, 0.33],
  RateLimit: [0.5, 0.225], // reuses Idle's sequence, so Idle's crop too
};

interface LoadedFrame {
  img: HTMLImageElement;
  ready: boolean;
}

export class TuxRenderer {
  private ctx: CanvasRenderingContext2D;
  private W: number;
  private H: number;
  private cx: number;
  private cy = 222;

  private sequences = new Map<string, LoadedFrame[]>();

  private currentState: SessionState = "Idle";
  private frameIndex = 0;
  private frameDir: 1 | -1 = 1;
  private frameChangedAt = 0;
  private bounceTween: { start: number; loop: boolean } | null = null;
  private tiltTween = new Tween(0, 0, 1);
  private squish = 0;
  private cursorX: number;
  private hasMouse = false;
  private lastZzzAt = 0;
  private lastSweatAt = 0;
  private burstDone = false;
  private avatarMode: boolean;

  /** `avatarMode` — used by the notch's small circular icon (PRD session
   * 3: reported live as imprecise/cropped-looking): the full-body sprite
   * squeezed into a 56px circle clipped off the head, since the sprite's
   * own vertical center is roughly torso-height, not face-height. Zooms
   * in and re-centers on the face instead of the whole body, and skips
   * the glow/badge chrome that's meaningless at that size. */
  constructor(canvas: HTMLCanvasElement, opts: { avatarMode?: boolean } = {}) {
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("2d context unavailable");
    this.ctx = ctx;
    this.W = canvas.width;
    this.H = canvas.height;
    this.cx = this.W / 2;
    this.cursorX = this.cx;
    this.avatarMode = opts.avatarMode ?? false;

    // Multiple states can point at the same frame directory (RateLimit ->
    // idle), so load each distinct directory's sequence once and share it.
    for (const dir of new Set(Object.values(FRAME_DIR))) {
      this.sequences.set(dir, this.loadSequence(dir, FRAME_COUNT[dir]));
    }

    canvas.addEventListener("mousemove", (e) => {
      const r = canvas.getBoundingClientRect();
      this.cursorX = e.clientX - r.left;
      this.hasMouse = true;
    });
    canvas.addEventListener("mouseleave", () => {
      this.hasMouse = false;
    });
    canvas.addEventListener("click", () => {
      this.squish = 1;
      this.bounceTween = { start: performance.now(), loop: false };
    });
  }

  private loadSequence(dir: string, count: number): LoadedFrame[] {
    const frames: LoadedFrame[] = [];
    for (let i = 0; i < count; i++) {
      const img = new Image();
      const entry: LoadedFrame = { img, ready: false };
      img.onload = () => {
        entry.ready = true;
      };
      img.src = `/src/assets/states/${dir}/f${String(i).padStart(3, "0")}.png`;
      frames.push(entry);
    }
    return frames;
  }

  setState(name: SessionState) {
    this.currentState = name;
    this.burstDone = false;
    this.frameIndex = 0;
    this.frameDir = 1;
    this.frameChangedAt = performance.now();
    this.tiltTween = new Tween(this.tiltTween.value(), STATE[name].tilt, 350, Ease.inOut);
    this.bounceTween = STATE[name].bounce ? { start: performance.now(), loop: true } : null;
  }

  getState(): SessionState {
    return this.currentState;
  }

  private hexA(hex: string, a: number): string {
    const n = parseInt(hex.slice(1), 16);
    const r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
    return `rgba(${r},${g},${b},${a})`;
  }

  private drawBadge(cfg: (typeof STATE)[SessionState], x: number, y: number) {
    const ctx = this.ctx;
    if (!cfg.badge) return;
    ctx.save();
    ctx.beginPath();
    ctx.arc(x, y, 16, 0, Math.PI * 2);
    ctx.fillStyle = cfg.color;
    ctx.fill();
    ctx.strokeStyle = "#0c0e12";
    ctx.lineWidth = 3;
    ctx.stroke();
    ctx.fillStyle = "#fff";
    ctx.font = "700 16px sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    if (cfg.badge === "bang") ctx.fillText("!", x, y + 1);
    else if (cfg.badge === "question") ctx.fillText("?", x, y + 1);
    else if (cfg.badge === "dots") {
      const now = performance.now();
      for (let i = -1; i <= 1; i++) {
        const p = 0.5 + 0.5 * Math.sin(now / 220 + i * 1.4);
        ctx.beginPath();
        ctx.arc(x + i * 5.5, y, 2 + p * 1.2, 0, Math.PI * 2);
        ctx.fill();
      }
    } else if (cfg.badge === "dot") {
      ctx.beginPath();
      ctx.arc(x, y, 3.5, 0, Math.PI * 2);
      ctx.fill();
    }
    ctx.restore();
  }

  private handleTimedEffects(now: number, cfg: (typeof STATE)[SessionState], spriteW: number) {
    if (cfg.burst && !this.burstDone) {
      this.burstDone = true;
      for (let i = 0; i < 10; i++) {
        const type = Math.random() < 0.5 ? "heart" : "star";
        spawnParticle(type, this.cx + (Math.random() * 2 - 1) * 60, this.cy - SPRITE_H * 0.3, {
          vx: (Math.random() * 2 - 1) * 40,
          vy: -60 - Math.random() * 40,
          life: 1.1 + Math.random() * 0.5,
        });
      }
    }
    if (cfg.tired) {
      if (now - this.lastZzzAt > 900) {
        this.lastZzzAt = now;
        spawnParticle("zzz", this.cx + spriteW * 0.32, this.cy - SPRITE_H * 0.38, {
          vx: 6,
          vy: -22,
          life: 1.6,
          size: 14,
        });
      }
      if (now - this.lastSweatAt > 2200) {
        this.lastSweatAt = now;
        spawnParticle("sweat", this.cx - spriteW * 0.36, this.cy - SPRITE_H * 0.25, {
          vx: 2,
          vy: 30,
          life: 0.9,
          size: 10,
        });
      }
    }
  }

  /** Ping-pongs frameIndex across the sequence at FRAME_MS pace — a hard
   * loop (jump back to frame 0) would pop visibly since these clips were
   * cropped from mid-gesture video, not authored as seamless loops. */
  private advanceFrame(now: number, frameCount: number) {
    if (frameCount <= 1) return;
    let steps = Math.floor((now - this.frameChangedAt) / FRAME_MS);
    if (steps <= 0) return;
    steps = Math.min(steps, 5);
    for (let i = 0; i < steps; i++) {
      this.frameIndex += this.frameDir;
      if (this.frameIndex >= frameCount - 1) {
        this.frameIndex = frameCount - 1;
        this.frameDir = -1;
      } else if (this.frameIndex <= 0) {
        this.frameIndex = 0;
        this.frameDir = 1;
      }
    }
    this.frameChangedAt += steps * FRAME_MS;
  }

  frame(now: number) {
    const ctx = this.ctx;
    const cfg = STATE[this.currentState];

    const breathe = Math.sin((now / 1000) * (cfg.tired ? 0.9 : 1.6)) * (cfg.tired ? 1.5 : 2.5);

    let bounceY = 0;
    if (this.bounceTween) {
      const bt = (now - this.bounceTween.start) / 550;
      const phase = this.bounceTween.loop ? bt % 1 : Math.min(1, bt);
      bounceY = -Math.abs(Math.sin(phase * Math.PI)) * 12 * Ease.out(Math.min(1, bt * 3));
    }

    this.squish *= 0.9;
    const shakeX = cfg.shake ? Math.sin((now / 1000) * 28) * 2.5 : 0;
    const wobble = cfg.wobble ? Math.sin((now / 1000) * 3.5) * 0.05 : 0;
    const lookTilt = this.hasMouse
      ? Math.max(-1, Math.min(1, (this.cursorX - this.cx) / (this.W / 2))) * 0.06
      : 0;
    const tilt = this.tiltTween.value(now) + wobble + lookTilt;

    ctx.clearRect(0, 0, this.W, this.H);

    if (!this.avatarMode) {
      const glow = ctx.createRadialGradient(this.cx, this.cy, 20, this.cx, this.cy, 230);
      glow.addColorStop(0, this.hexA(cfg.glow, cfg.glowOp));
      glow.addColorStop(1, this.hexA(cfg.glow, 0));
      ctx.fillStyle = glow;
      ctx.fillRect(0, 0, this.W, this.H);
    }

    const dirKey = FRAME_DIR[this.currentState];
    const frames = this.sequences.get(dirKey) ?? [];
    this.advanceFrame(now, frames.length);
    const entry = frames[this.frameIndex] as LoadedFrame | undefined;
    const spriteW = entry?.ready ? SPRITE_H * (entry.img.naturalWidth / entry.img.naturalHeight) : SPRITE_H;

    ctx.save();
    ctx.translate(this.cx + shakeX, this.cy + breathe + bounceY);
    ctx.rotate(tilt);
    const squishY = 1 - this.squish * 0.14, squishX = 1 + this.squish * 0.11;
    ctx.scale(squishX, squishY);

    if (entry?.ready) {
      if (this.avatarMode) {
        // A single Idle-calibrated crop clipped badly on the ~5 states
        // whose source scene is a wide landscape shot (character off to
        // one side next to a prop) rather than Idle's portrait framing —
        // reported live ("Working" showed just a sliver of laptop).
        // AVATAR_FACE_FRAC has a crop calibrated per state instead.
        const [faceFracX, faceFracY] = AVATAR_FACE_FRAC[this.currentState];
        const h = SPRITE_H * AVATAR_ZOOM;
        const w = spriteW * AVATAR_ZOOM;
        ctx.drawImage(entry.img, -faceFracX * w, -faceFracY * h, w, h);
      } else {
        ctx.drawImage(entry.img, -spriteW / 2, -SPRITE_H / 2, spriteW, SPRITE_H);
      }
    }
    if (!this.avatarMode) this.drawBadge(cfg, spriteW * 0.3, -SPRITE_H * 0.42);

    ctx.restore();

    updateParticles(1 / 60);
    drawParticles(ctx);
    this.handleTimedEffects(now, cfg, spriteW);
  }
}
