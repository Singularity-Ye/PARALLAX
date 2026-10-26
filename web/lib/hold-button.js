/**
 * Vanilla HoldButton (ported from React Bits Micro/HoldButton)
 * Press & hold for satisfying tactile arcade charge-up (450ms) with spring reset.
 * Fast taps (<220ms) still fire normally so quick users aren't blocked.
 */
export function initHoldButton(selector = '#start, [data-hold]', options = {}) {
  if (typeof window === 'undefined') return;

  const HOLD_DURATION = options.holdTime || 450;
  const TAP_MAX_MS = 220;

  const buttons = document.querySelectorAll(selector);

  buttons.forEach(btn => {
    if (btn.dataset.holdInitialized) return;
    btn.dataset.holdInitialized = 'true';
    btn.classList.add('hold-btn');

    // Ensure inner structure
    const origHtml = btn.innerHTML;
    btn.innerHTML = `
      <span class="hold-fill" aria-hidden="true"></span>
      <span class="hold-content">${origHtml}</span>
      <span class="hold-meter" aria-hidden="true"><span class="hold-dot"></span></span>
    `;

    let start = 0;
    let rafId = 0;
    let completed = false;
    let phase = 'idle'; // idle | holding | complete

    function setPhase(next) {
      phase = next;
      btn.dataset.phase = next;
    }

    function cancelHold(spring = true) {
      if (rafId) cancelAnimationFrame(rafId);
      rafId = 0;
      if (spring && phase === 'holding') {
        const cur = parseFloat(btn.style.getPropertyValue('--hb-p') || '0');
        const resetStart = performance.now();
        const resetStep = (now) => {
          const t = Math.min(1, (now - resetStart) / 180);
          const p = cur * (1 - t * t);
          btn.style.setProperty('--hb-p', p.toFixed(3));
          if (t < 1) {
            rafId = requestAnimationFrame(resetStep);
          } else {
            btn.style.setProperty('--hb-p', '0');
            setPhase('idle');
            rafId = 0;
          }
        };
        rafId = requestAnimationFrame(resetStep);
      } else {
        btn.style.setProperty('--hb-p', '0');
        setPhase('idle');
      }
    }

    function onPointerDown(e) {
      if (e.button !== 0 || btn.disabled) return;
      start = performance.now();
      completed = false;
      setPhase('holding');

      const step = (now) => {
        const elapsed = now - start;
        const p = Math.min(1, elapsed / HOLD_DURATION);
        btn.style.setProperty('--hb-p', p.toFixed(3));

        if (p < 1) {
          rafId = requestAnimationFrame(step);
        } else {
          completed = true;
          setPhase('complete');
          btn.style.setProperty('--hb-p', '1');
          
          // Trigger click / submit
          btn.dispatchEvent(new CustomEvent('hold:complete', { bubbles: true }));
          
          if (btn.type === 'submit' && btn.form) {
            btn.form.requestSubmit(btn);
          } else {
            btn.click();
          }

          setTimeout(() => {
            cancelHold(false);
          }, 600);
        }
      };

      rafId = requestAnimationFrame(step);
    }

    function onPointerUp(e) {
      if (phase !== 'holding') return;
      const elapsed = performance.now() - start;

      if (!completed && elapsed < TAP_MAX_MS) {
        // Fast tap: fire immediately!
        cancelHold(false);
        if (btn.type === 'submit' && btn.form) {
          btn.form.requestSubmit(btn);
        } else {
          btn.click();
        }
      } else if (!completed) {
        cancelHold(true);
      }
    }

    function onPointerLeave() {
      if (phase === 'holding' && !completed) {
        cancelHold(true);
      }
    }

    btn.addEventListener('pointerdown', onPointerDown);
    btn.addEventListener('pointerup', onPointerUp);
    btn.addEventListener('pointerleave', onPointerLeave);
    btn.addEventListener('pointercancel', onPointerLeave);
  });
}
