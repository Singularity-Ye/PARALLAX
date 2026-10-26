/**
 * Ultra-lightweight Mechanical Split-Flap Ticker
 * Zero external dependencies. Uses CSS translateY & spring-soft easing.
 */
export function renderTicker(container, text) {
  const str = String(text ?? '');
  if (!container) return;

  const reducedMotion = typeof window !== 'undefined' && 
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

  if (reducedMotion) {
    container.textContent = str;
    return;
  }

  container.replaceChildren();
  const digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

  let digitIndex = 0;
  for (const char of str) {
    if (char >= '0' && char <= '9') {
      const slot = document.createElement('span');
      slot.className = 'ticker-slot';
      
      const strip = document.createElement('span');
      strip.className = 'ticker-strip';
      strip.style.transitionDelay = `${digitIndex * 45}ms`;
      digitIndex++;

      for (const d of digits) {
        const s = document.createElement('span');
        s.textContent = d;
        strip.append(s);
      }

      const val = parseInt(char, 10);
      strip.style.transform = 'translateY(0%)';
      slot.append(strip);
      container.append(slot);

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          strip.style.transform = `translateY(-${val * 10}%)`;
        });
      });
    } else {
      const sym = document.createElement('span');
      sym.className = 'ticker-sym';
      sym.textContent = char;
      container.append(sym);
    }
  }
}
