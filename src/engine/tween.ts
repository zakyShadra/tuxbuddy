import { Ease, type EaseFn } from "./easing";

export class Tween {
  private from: number;
  private to: number;
  private duration: number;
  private ease: EaseFn;
  private start: number;

  constructor(from: number, to: number, durationMs: number, ease: EaseFn = Ease.out) {
    this.from = from;
    this.to = to;
    this.duration = durationMs;
    this.ease = ease;
    this.start = performance.now();
  }

  value(now: number = performance.now()): number {
    const t = Math.min(1, (now - this.start) / this.duration);
    return this.from + (this.to - this.from) * this.ease(t);
  }
}
