const FRAME_MS = 1000 / 24;
const SPRITE_H = 200;

/**
 * The roaming pet window's renderer — deliberately much simpler than
 * `renderTux.ts` (the notch/status renderer): no glow, badge, or particle
 * effects, since PRD session 3 moved all of that to the notch bar. This
 * one only ever shows Idle / Walk / Run / Stop / Flying — the ambient
 * "desktop pet" poses — driven by `roaming.rs`'s `tuxbuddy://roam-motion`
 * event (see `main.ts`) rather than `SessionState`.
 */
// jump/peek/surprised are the batch2.mp4 "gestures" processed in PRD §5
// but left unwired until now (§4.4 #11) — random idle-variety beats so a
// long idle roam doesn't just breathe in place forever. `descend` from
// the same batch is used for `playEntrance` (below) instead — the
// desktop-follow arrival flourish.
type PetPose = "idle" | "walk" | "run" | "stop" | "jump" | "peek" | "surprised" | "descend";
const IDLE_GESTURES: PetPose[] = ["jump", "peek", "surprised"];

// idle reuses the existing states/idle sequence (shared with the notch
// window's RateLimit pose); walk/run/stop are the session-3 roam/*
// sequences (src/assets/roam/<name>/f000.png..).
const POSE_DIR: Record<PetPose, string> = {
  idle: "states/idle",
  walk: "roam/walk",
  run: "roam/run",
  stop: "roam/stop",
  jump: "gestures/jump",
  peek: "gestures/peek",
  surprised: "gestures/surprised",
  descend: "gestures/descend",
};

const FRAME_COUNT: Record<PetPose, number> = {
  idle: 87,
  // Trimmed from the source's 36 frames to 20 — frames ~20-35 turn the
  // character to face the camera (part of the original walk->run->stop
  // performance's transition beat), which read as a jarring head-turn
  // when looped as a side-profile walk cycle. Reported live: "jalannya
  // masih sering nengok-nengok" — re-cropped from the same rembg output,
  // see PRD §4.1c point 3 / §5.
  walk: 20,
  run: 101,
  stop: 61,
  jump: 48,
  peek: 38,
  surprised: 63,
  descend: 53,
};

const FLYING_DIR = "flying";
const FLYING_FRAME_COUNT = 60;

interface LoadedFrame {
  img: HTMLImageElement;
  ready: boolean;
}

export class PetRenderer {
  private ctx: CanvasRenderingContext2D;
  private W: number;
  private H: number;
  private cx: number;
  private cy: number;

  private sequences = new Map<string, LoadedFrame[]>();

  private pose: PetPose = "idle";
  private facing: 1 | -1 = 1;
  private frameIndex = 0;
  private frameDir: 1 | -1 = 1;
  private frameChangedAt = 0;
  private breatheAmp = 2.5;

  private flying = false;
  private flyDone: (() => void) | null = null;

  /** Next time an idle-variety gesture (jump/peek/surprised) is allowed
   * to fire — reset to a fresh random delay whenever idle actually
   * (re)starts, so gestures don't fire back-to-back or the instant a walk
   * interruption ends. */
  private nextGestureAt = performance.now() + this.randomGestureDelay();

  /** Plays once then falls back to the current pose (used for the one-shot
   * skid-to-halt when roaming stops — see `setMoving`). */
  private onceDone: (() => void) | null = null;

  constructor(canvas: HTMLCanvasElement) {
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("2d context unavailable");
    this.ctx = ctx;
    this.W = canvas.width;
    this.H = canvas.height;
    this.cx = this.W / 2;
    this.cy = this.H / 2 + 20;

    for (const [name, count] of Object.entries(FRAME_COUNT) as [PetPose, number][]) {
      this.sequences.set(name, this.loadSequence(POSE_DIR[name], count));
    }
    this.sequences.set(FLYING_DIR, this.loadSequence("states/" + FLYING_DIR, FLYING_FRAME_COUNT));
  }

  private loadSequence(dir: string, count: number): LoadedFrame[] {
    const frames: LoadedFrame[] = [];
    for (let i = 0; i < count; i++) {
      const img = new Image();
      const entry: LoadedFrame = { img, ready: false };
      img.onload = () => {
        entry.ready = true;
      };
      img.src = `/src/assets/${dir}/f${String(i).padStart(3, "0")}.png`;
      frames.push(entry);
    }
    return frames;
  }

  /** Driven by `tuxbuddy://roam-motion` — switches between the idle
   * standing loop and the walk cycle, and flips to face the direction of
   * travel. A falling edge (was moving, now stopped) plays the skid-stop
   * clip once before settling into idle. */
  setMoving(moving: boolean, dir: 1 | -1) {
    this.facing = dir;
    if (moving) {
      if (this.pose !== "walk") this.setPose("walk");
      return;
    }
    if (this.pose === "walk" || this.pose === "run") {
      this.playOnce("stop", () => this.setPose("idle"));
    } else if (this.pose !== "idle" && !this.flying) {
      this.setPose("idle");
    }
  }

  private randomGestureDelay(): number {
    return 15000 + Math.random() * 15000; // 15-30s
  }

  private setPose(pose: PetPose) {
    this.pose = pose;
    this.frameIndex = 0;
    this.frameDir = 1;
    this.frameChangedAt = performance.now();
    this.onceDone = null;
    if (pose === "idle") this.nextGestureAt = performance.now() + this.randomGestureDelay();
  }

  /** Idle-only: occasionally plays a random jump/peek/surprised beat so a
   * long idle roam doesn't just stand there breathing forever. */
  private maybePlayIdleGesture(now: number) {
    if (this.pose !== "idle" || this.flying || now < this.nextGestureAt) return;
    const gesture = IDLE_GESTURES[Math.floor(Math.random() * IDLE_GESTURES.length)];
    this.playOnce(gesture, () => this.setPose("idle"));
  }

  private playOnce(pose: PetPose, onDone: () => void) {
    this.pose = pose;
    this.frameIndex = 0;
    this.frameDir = 1;
    this.frameChangedAt = performance.now();
    this.onceDone = onDone;
  }

  /** Plays the takeoff->glide->land clip once (attention/fly-to-user
   * flow), then calls onDone. See PRD §2.1's required order: fly in, stop,
   * *then* show any UI. */
  playFlyIn(onDone: () => void) {
    this.flying = true;
    this.flyDone = onDone;
    this.frameIndex = 0;
    this.frameDir = 1;
    this.frameChangedAt = performance.now();
  }

  /** Desktop-follow arrival flourish (PRD session-3: "pas mau jalan ke
   * desktop lain tuh dia pake animasi ngintip dulu, baru masuk"). Just
   * descend/dive-in -> idle — an earlier version chained on a run+jump
   * tail ("terbang+landing, dan run+lompat"), but the user called that
   * combo out live as not flowing together ("ngga nyambung"), so it's
   * back to the single clean beat. `run`/`jump` stay available as
   * standalone poses for whatever needs them next (roaming/idle-gesture
   * already use them elsewhere). */
  playEntrance(onDone: () => void) {
    this.playOnce("descend", () => {
      this.setPose("idle");
      onDone();
    });
  }

  private currentDir(): string {
    if (this.flying) return FLYING_DIR;
    return this.pose;
  }

  /** Ping-pongs every pose, walk/run included — an initial version hard-
   * looped walk/run (jump straight back to frame 0), which popped visibly
   * every cycle since frame 0 and the last frame don't match exactly
   * (these are cropped from a continuous take, not an authored loop).
   * Reported live as "jalannya masih patah-patah" — reversing instead of
   * cutting reads as a natural waddle/sway rather than a stride, since
   * these clips are mostly a body-rock, not a full walking gait. */
  private advanceFrame(now: number, frameCount: number) {
    if (frameCount <= 1) return;
    let steps = Math.floor((now - this.frameChangedAt) / FRAME_MS);
    if (steps <= 0) return;
    steps = Math.min(steps, 5);
    for (let i = 0; i < steps; i++) {
      if (this.flying && this.frameIndex >= frameCount - 1) {
        this.flying = false;
        const done = this.flyDone;
        this.flyDone = null;
        this.frameIndex = 0;
        this.frameDir = 1;
        done?.();
        break;
      }
      if (!this.flying && this.onceDone && this.frameIndex >= frameCount - 1) {
        const done = this.onceDone;
        this.onceDone = null;
        done();
        break;
      }
      this.frameIndex += this.frameDir;
      if (this.frameIndex >= frameCount - 1) {
        this.frameIndex = frameCount - 1;
        if (!this.flying && !this.onceDone) this.frameDir = -1;
      } else if (this.frameIndex <= 0) {
        this.frameIndex = 0;
        this.frameDir = 1;
      }
    }
    this.frameChangedAt += steps * FRAME_MS;
  }

  /** Placeholder doomscroll-nudge gesture (PRD §4.4 behavior #3, first
   * pass) — "klo doom scroll kasih animasi ketuk layar aja dulu, nanti
   * aku kasih animasi yang sesuai": no real footage for this exists yet,
   * so it's a small procedural squish/shake layered on top of whatever
   * pose is already playing, the same trick `renderTux.ts` uses for its
   * click-squish feedback, rather than a dedicated frame sequence. Swap
   * this out for a real clip later without touching the trigger plumbing
   * (`main.ts`'s `listenForTap`) at all.
   */
  playTap() {
    this.tapPulse = 1;
  }

  private tapPulse = 0;

  frame(now: number) {
    const ctx = this.ctx;
    this.maybePlayIdleGesture(now);
    const breathe = this.pose === "idle" ? Math.sin((now / 1000) * 1.6) * this.breatheAmp : 0;

    this.tapPulse *= 0.88;
    const tapShakeX = this.tapPulse > 0.02 ? Math.sin(now / 30) * 6 * this.tapPulse : 0;
    const tapSquish = this.tapPulse * 0.16;

    ctx.clearRect(0, 0, this.W, this.H);

    const dirKey = this.currentDir();
    const frames = this.sequences.get(dirKey) ?? [];
    this.advanceFrame(now, frames.length);
    const entry = frames[this.frameIndex] as LoadedFrame | undefined;
    const spriteW = entry?.ready ? SPRITE_H * (entry.img.naturalWidth / entry.img.naturalHeight) : SPRITE_H;

    ctx.save();
    ctx.translate(this.cx + tapShakeX, this.cy + breathe);
    ctx.scale(this.facing * (1 + tapSquish), 1 - tapSquish);

    if (entry?.ready) {
      ctx.drawImage(entry.img, -spriteW / 2, -SPRITE_H / 2, spriteW, SPRITE_H);
    }

    ctx.restore();
  }
}
