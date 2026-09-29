export type ParticleType = "heart" | "star" | "sweat" | "zzz";

interface Particle {
  type: ParticleType;
  x: number;
  y: number;
  vx: number;
  vy: number;
  age: number;
  life: number;
  rot: number;
  size: number;
}

let particles: Particle[] = [];

export function spawnParticle(
  type: ParticleType,
  x: number,
  y: number,
  opts: Partial<Pick<Particle, "vx" | "vy" | "life" | "size">> = {},
) {
  particles.push({
    type,
    x,
    y,
    vx: opts.vx ?? (Math.random() * 2 - 1) * 18,
    vy: opts.vy ?? -(30 + Math.random() * 30),
    age: 0,
    life: opts.life ?? 1.1,
    rot: 0,
    size: opts.size ?? 10 + Math.random() * 6,
  });
}

export function updateParticles(dt: number) {
  for (const p of particles) {
    p.age += dt;
    p.x += p.vx * dt;
    p.y += p.vy * dt;
    p.vy += 40 * dt;
    p.rot += dt * 1.2;
  }
  particles = particles.filter((p) => p.age < p.life);
}

function drawHeart(ctx: CanvasRenderingContext2D, s: number) {
  ctx.beginPath();
  ctx.moveTo(0, s * 0.3);
  ctx.bezierCurveTo(-s, -s * 0.4, -s * 0.5, -s, 0, -s * 0.15);
  ctx.bezierCurveTo(s * 0.5, -s, s, -s * 0.4, 0, s * 0.3);
  ctx.closePath();
  ctx.fill();
}

function drawStar(ctx: CanvasRenderingContext2D, s: number) {
  ctx.beginPath();
  for (let i = 0; i < 5; i++) {
    const a = -Math.PI / 2 + i * ((2 * Math.PI) / 5);
    const a2 = a + Math.PI / 5;
    ctx.lineTo(Math.cos(a) * s, Math.sin(a) * s);
    ctx.lineTo(Math.cos(a2) * s * 0.45, Math.sin(a2) * s * 0.45);
  }
  ctx.closePath();
  ctx.fill();
}

function drawDrop(ctx: CanvasRenderingContext2D, s: number) {
  ctx.beginPath();
  ctx.arc(0, s * 0.2, s * 0.55, 0, Math.PI * 2);
  ctx.moveTo(-s * 0.5, s * 0.15);
  ctx.quadraticCurveTo(0, -s * 1.1, s * 0.5, s * 0.15);
  ctx.fill();
}

export function drawParticles(ctx: CanvasRenderingContext2D) {
  for (const p of particles) {
    const t = p.age / p.life;
    ctx.save();
    ctx.globalAlpha = Math.max(0, 1 - t);
    ctx.translate(p.x, p.y);
    ctx.rotate(p.rot * (p.type === "zzz" ? 0 : 0.3));
    const s = p.size * (1 - t * 0.3);
    if (p.type === "heart") {
      ctx.fillStyle = "#ff5d7a";
      drawHeart(ctx, s);
    } else if (p.type === "star") {
      ctx.fillStyle = "#ffd23f";
      drawStar(ctx, s);
    } else if (p.type === "sweat") {
      ctx.fillStyle = "#7ec8ff";
      drawDrop(ctx, s);
    } else if (p.type === "zzz") {
      ctx.fillStyle = "#c7cbd4";
      ctx.font = `700 ${s + 4}px sans-serif`;
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("z", 0, 0);
    }
    ctx.restore();
  }
}
