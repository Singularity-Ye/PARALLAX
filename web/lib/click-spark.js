import {motionScope} from './motion.js';
/**
 * Vanilla ClickSpark (adapted from React Bits Animations/ClickSpark)
 * Emits crisp, neon-green retro sparks/stars on click.
 * Single canvas; requests frames only while sparks remain.
 */
export function initClickSpark(options = {}) {
  if (typeof window === 'undefined') return;
  const scope=motionScope();

  const config = {
    colors: options.colors || ['#b5ff38', '#ffffff', '#8be000'],
    size: options.size || 10,
    radius: options.radius || 28,
    count: options.count || 8,
    duration: options.duration || 320,
    ...options
  };

  const canvas = document.createElement('canvas');
  canvas.id = 'parallax-click-spark';
  canvas.style.cssText = 'position:fixed;inset:0;width:100vw;height:100vh;pointer-events:none;z-index:99999;';
  document.body.appendChild(canvas);

  const ctx = canvas.getContext('2d');
  if(!ctx){canvas.remove();return ()=>scope.dispose();}
  scope.cleanup(()=>canvas.remove());
  let bursts = [];
  let rafId = 0;

  function resize() {
    const dpr = Math.min(window.devicePixelRatio || 1,2);
    canvas.width = window.innerWidth * dpr;
    canvas.height = window.innerHeight * dpr;
    ctx.scale(dpr, dpr);
  }
  resize();
  scope.on(window,'resize', resize, { passive: true });

  // 4-pointed retro star shape (matches CS2 / Neo-Brutalist ✦ star)
  function drawStar(ctx, x, y, radius, rotation) {
    const pinch = radius * 0.22;
    ctx.beginPath();
    ctx.moveTo(x + Math.cos(rotation) * radius, y + Math.sin(rotation) * radius);
    for (let i = 1; i <= 4; i++) {
      const a = rotation + (i * Math.PI) / 2;
      const mid = a - Math.PI / 4;
      ctx.quadraticCurveTo(
        x + Math.cos(mid) * pinch,
        y + Math.sin(mid) * pinch,
        x + Math.cos(a) * radius,
        y + Math.sin(a) * radius
      );
    }
    ctx.closePath();
    ctx.fill();
  }

  function addBurst(x, y) {
    const sparks = [];
    for (let i = 0; i < config.count; i++) {
      const angle = (i * 2 * Math.PI) / config.count + (Math.random() - 0.5) * 0.4;
      const speed = 0.7 + Math.random() * 0.6;
      sparks.push({
        angle,
        speed,
        rot: Math.random() * Math.PI,
        rotSpeed: (Math.random() - 0.5) * 8,
        color: config.colors[Math.floor(Math.random() * config.colors.length)],
        size: config.size * (0.6 + Math.random() * 0.7)
      });
    }
    bursts.push({ x, y, start: performance.now(), sparks });
    if (!rafId) rafId = scope.frame(loop);
  }

  function loop(now) {
    ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
    const aliveBursts = [];

    for (const burst of bursts) {
      const elapsed = now - burst.start;
      const p = Math.min(1, elapsed / config.duration);
      if (p >= 1) continue;

      // Spring-like ease-out (1 - (1-p)^3)
      const ease = 1 - Math.pow(1 - p, 3);
      const alpha = Math.max(0, 1 - p * p);

      for (const s of burst.sparks) {
        const dist = config.radius * s.speed * ease;
        const curX = burst.x + Math.cos(s.angle) * dist;
        const curY = burst.y + Math.sin(s.angle) * dist;
        const curSize = Math.max(0, s.size * (1 - p * 0.5));

        ctx.save();
        ctx.globalAlpha = alpha;
        ctx.fillStyle = s.color;
        drawStar(ctx, curX, curY, curSize, s.rot + s.rotSpeed * p);
        ctx.restore();
      }
      aliveBursts.push(burst);
    }

    bursts = aliveBursts;
    if (bursts.length > 0) {
      rafId = scope.frame(loop);
    } else {
      rafId = 0;
      ctx.clearRect(0, 0, window.innerWidth, window.innerHeight);
    }
  }

  scope.on(window,'pointerdown', (e) => {
    // Trigger spark on clicks (primary button)
    if (e.button === 0) {
      addBurst(e.clientX, e.clientY);
    }
  }, { passive: true });
  return ()=>scope.dispose();
}
