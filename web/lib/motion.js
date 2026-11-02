// Shared ownership of listeners and animation frames; effects return one disposer.
export function motionScope() {
  const cleanups = [];
  const frames = new Set();
  return {
    on(target, type, listener, options) {
      target.addEventListener(type, listener, options);
      cleanups.push(() => target.removeEventListener(type, listener, options));
    },
    frame(callback) {
      const id = requestAnimationFrame(now => {
        frames.delete(id);
        callback(now);
      });
      frames.add(id);
      return id;
    },
    cancel(id) {
      cancelAnimationFrame(id);
      frames.delete(id);
    },
    cleanup(callback) { cleanups.push(callback); },
    dispose() {
      frames.forEach(cancelAnimationFrame);
      frames.clear();
      cleanups.reverse().forEach(callback => callback());
    }
  };
}

// Substeps keep spring integration stable. Seconds come from the rAF timestamp.
export function springStep(state, target, seconds, stiffness, damping) {
  const steps = Math.max(1, Math.ceil(seconds / (1 / 120)));
  const dt = seconds / steps;
  for (let i = 0; i < steps; i++) {
    state.v += (stiffness * (target - state.x) - damping * state.v) * dt;
    state.x += state.v * dt;
  }
  if (Math.abs(target - state.x) < 0.05 && Math.abs(state.v) < 0.05) {
    state.x = target;
    state.v = 0;
    return false;
  }
  return true;
}

export function frameClock() {
  let previous = null;
  return now => {
    // Start at rest; clamp delayed frames instead of catching up a hidden tab.
    const dt = previous === null ? 0 : Math.max(0, Math.min((now - previous) / 1000, 0.05));
    previous = now;
    return dt;
  };
}

export function initMotion(effects) {
  const preference = matchMedia('(prefers-reduced-motion: reduce)');
  let disposers = [];
  function stop() { disposers.splice(0).forEach(dispose => dispose?.()); }
  function sync() {
    stop();
    if (!preference.matches && !document.hidden) disposers = effects.map(init => init());
  }
  preference.addEventListener('change', sync);
  document.addEventListener('visibilitychange', sync);
  window.addEventListener('parallax:pagechange', sync);
  window.addEventListener('pagehide', stop);
  window.addEventListener('pageshow', sync);
  sync();
  return () => {
    stop();
    preference.removeEventListener('change', sync);
    document.removeEventListener('visibilitychange', sync);
    window.removeEventListener('parallax:pagechange', sync);
    window.removeEventListener('pagehide', stop);
    window.removeEventListener('pageshow', sync);
  };
}
