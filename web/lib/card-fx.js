/**
 * Parallax & Spotlight Card Effect (inspired by React Bits SpotlightCard & TiltedCard)
 * Zero external dependencies. Uses CSS variables, 3D perspective and radial glow tracking.
 */
export function initCardFx(selector = '.candidate, .launch-card, .paper-stat') {
  if (typeof window === 'undefined') return;
  const reducedMotion = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;
  if (reducedMotion) return;

  document.addEventListener('pointermove', event => {
    const card = event.target.closest(selector);
    if (!card) return;
    const rect = card.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const pctX = (x / rect.width) - 0.5;
    const pctY = (y / rect.height) - 0.5;
    card.style.setProperty('--spot-x', `${x.toFixed(1)}px`);
    card.style.setProperty('--spot-y', `${y.toFixed(1)}px`);
    card.style.setProperty('--tilt-x', `${(-pctY * 6.5).toFixed(2)}deg`);
    card.style.setProperty('--tilt-y', `${(pctX * 6.5).toFixed(2)}deg`);
  }, {passive: true});

  document.addEventListener('pointerout', event => {
    const card = event.target.closest(selector);
    if (!card) return;
    if (!card.contains(event.relatedTarget)) {
      card.style.setProperty('--tilt-x', '0deg');
      card.style.setProperty('--tilt-y', '0deg');
    }
  }, {passive: true});
}
