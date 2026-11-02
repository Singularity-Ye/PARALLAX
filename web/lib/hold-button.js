import {motionScope} from './motion.js';

// Decoration only: native click/submit is the single business activation path.
export function initHoldButton(selector = '#start, [data-hold]', options = {}) {
  const scope = motionScope();
  const duration = options.holdTime ?? 450;
  document.querySelectorAll(selector).forEach(button => {
    const fill = document.createElement('span');
    fill.className = 'hold-fill';
    fill.setAttribute('aria-hidden', 'true');
    button.prepend(fill);
    button.classList.add('hold-btn');
    let frame = 0;
    let start = 0;

    function reset() {
      scope.cancel(frame);
      frame = 0;
      button.style.removeProperty('--hb-p');
      delete button.dataset.phase;
    }
    function step(now) {
      if (button.disabled) { reset(); return; }
      const progress = Math.min(1, (now - start) / duration);
      button.style.setProperty('--hb-p', progress);
      button.dataset.phase = progress === 1 ? 'complete' : 'holding';
      if (progress < 1) frame = scope.frame(step);
    }
    scope.on(button, 'pointerdown', event => {
      if (event.button !== 0 || button.disabled) return;
      reset();
      start = performance.now();
      frame = scope.frame(step);
    });
    for (const type of ['pointerup', 'pointerleave', 'pointercancel', 'blur']) {
      scope.on(button, type, reset);
    }
    scope.cleanup(() => {
      reset();
      fill.remove();
      button.classList.remove('hold-btn');
    });
  });
  return () => scope.dispose();
}
