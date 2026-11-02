import {motionScope, springStep, frameClock} from './motion.js';

export function initMagnet(selector = '[data-magnet], .brand-mark, .button-link, #start, #guard-copy') {
  const scope = motionScope();
  const states = new Map();
  let frame = 0;
  let pointer = null;
  let clock = frameClock();

  function wake() {
    if (!frame) {
      clock = frameClock();
      frame = scope.frame(step);
    }
  }
  function step(now) {
    frame = 0;
    const dt = clock(now);
    let closest = null;
    let distance = Infinity;
    if (pointer) {
      // Pointer events only update coordinates; layout is read once per frame.
      for (const element of document.querySelectorAll(selector)) {
        if (element.disabled || element.offsetParent === null) continue;
        const rect = element.getBoundingClientRect();
        const dx = pointer.x - rect.left - rect.width / 2;
        const dy = pointer.y - rect.top - rect.height / 2;
        const d = Math.hypot(dx, dy);
        if (d < distance && d < Math.max(rect.width, rect.height) / 2 + 60) {
          closest = {element, dx, dy, d, rect};
          distance = d;
        }
      }
    }
    if (closest && !states.has(closest.element)) {
      states.set(closest.element, {x: {x: 0, v: 0}, y: {x: 0, v: 0}});
    }
    let moving = false;
    for (const [element, state] of states) {
      let tx = 0;
      let ty = 0;
      if (closest?.element === element) {
        const pull = Math.max(0, 1 - closest.d / (closest.rect.width / 2 + 60));
        tx = closest.dx / 3.5 * pull;
        ty = closest.dy / 3.5 * pull;
        const length = Math.hypot(tx, ty);
        if (length > 14) { tx *= 14 / length; ty *= 14 / length; }
      }
      // All previous targets return to rest, including direct A -> B transitions.
      const movingX = springStep(state.x, tx, dt, 180, 14);
      const movingY = springStep(state.y, ty, dt, 180, 14);
      moving = movingX || movingY || moving;
      // Independent translate leaves CSS hover rotation/scale intact.
      element.style.translate = `${state.x.x}px ${state.y.x}px`;
    }
    if (moving) frame = scope.frame(step);
  }
  scope.on(window, 'pointermove', event => {
    if (event.pointerType === 'touch') return;
    pointer = {x: event.clientX, y: event.clientY};
    wake();
  }, {passive: true});
  function leave() { pointer = null; wake(); }
  scope.on(document.documentElement, 'pointerleave', leave);
  scope.on(window, 'blur', leave);
  scope.cleanup(() => states.forEach((_, element) => element.style.removeProperty('translate')));
  return () => scope.dispose();
}
