/**
 * Vanilla Magnet Effect (ported directly from React Bits Animations/Magnet)
 * Physics-based magnetic attraction that gently pulls buttons/badges toward the pointer.
 * Zero external dependencies. Self-sleeping RAF loop (0% idle CPU).
 */
export function initMagnet(selector = '[data-magnet], .brand-mark, .button-link, #start, #guard-copy') {
  if (typeof window === 'undefined') return;
  const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  if (reduced) return;

  const PADDING = 60;      // trigger distance in px
  const STRENGTH = 3.5;    // resistance divisor (higher = subtler pull)
  const MAX_OFFSET = 14;   // maximum displacement in px

  let pointer = null;
  let activeElement = null;
  let rafId = 0;

  // Spring state per element
  const states = new WeakMap();

  function getState(el) {
    let s = states.get(el);
    if (!s) {
      s = { x: 0, y: 0, vx: 0, vy: 0, targetX: 0, targetY: 0 };
      states.set(el, s);
    }
    return s;
  }

  function onPointerMove(e) {
    if (e.pointerType === 'touch') return;
    pointer = { x: e.clientX, y: e.clientY };

    // Find closest element
    const elements = document.querySelectorAll(selector);
    let closest = null;
    let minDistance = Infinity;

    for (const el of elements) {
      if (el.disabled || el.hidden || el.offsetParent === null) continue;
      const rect = el.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const dx = pointer.x - cx;
      const dy = pointer.y - cy;
      const dist = Math.hypot(dx, dy);

      if (dist < minDistance) {
        minDistance = dist;
        closest = { el, rect, cx, cy, dx, dy, dist };
      }
    }

    if (closest && closest.dist < Math.max(closest.rect.width, closest.rect.height) / 2 + PADDING) {
      activeElement = closest.el;
      const s = getState(closest.el);
      const pull = Math.min(1, Math.max(0, 1 - (closest.dist / (closest.rect.width / 2 + PADDING))));
      let tx = (closest.dx / STRENGTH) * pull;
      let ty = (closest.dy / STRENGTH) * pull;

      const len = Math.hypot(tx, ty);
      if (len > MAX_OFFSET) {
        tx *= MAX_OFFSET / len;
        ty *= MAX_OFFSET / len;
      }
      s.targetX = tx;
      s.targetY = ty;
      wake();
    } else if (activeElement) {
      const s = getState(activeElement);
      s.targetX = 0;
      s.targetY = 0;
      activeElement = null;
      wake();
    }
  }

  function onLeave() {
    pointer = null;
    if (activeElement) {
      const s = getState(activeElement);
      s.targetX = 0;
      s.targetY = 0;
      activeElement = null;
      wake();
    }
  }

  function step() {
    rafId = 0;
    let anyMoving = false;
    const elements = document.querySelectorAll(selector);

    for (const el of elements) {
      const s = states.get(el);
      if (!s) continue;

      // Spring physics (stiffness 180, damping 14)
      const ax = 180 * (s.targetX - s.x) - 14 * s.vx;
      const ay = 180 * (s.targetY - s.y) - 14 * s.vy;
      s.vx += ax * (1 / 60);
      s.vy += ay * (1 / 60);
      s.x += s.vx * (1 / 60);
      s.y += s.vy * (1 / 60);

      if (Math.abs(s.targetX - s.x) > 0.05 || Math.abs(s.vx) > 0.05 ||
          Math.abs(s.targetY - s.y) > 0.05 || Math.abs(s.vy) > 0.05) {
        anyMoving = true;
      } else {
        s.x = s.targetX;
        s.y = s.targetY;
        s.vx = 0;
        s.vy = 0;
      }

      el.style.transform = (s.x === 0 && s.y === 0) ? '' : `translate3d(${s.x.toFixed(2)}px, ${s.y.toFixed(2)}px, 0)`;
    }

    if (anyMoving) {
      rafId = requestAnimationFrame(step);
    }
  }

  function wake() {
    if (!rafId) rafId = requestAnimationFrame(step);
  }

  window.addEventListener('pointermove', onPointerMove, { passive: true });
  document.documentElement.addEventListener('pointerleave', onLeave);
  window.addEventListener('blur', onLeave);
}
