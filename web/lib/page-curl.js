/**
 * Vanilla SVG Bézier Page Curl (柔韧书页矢量弧面卷角)
 * Zero external libraries. Hardware-accelerated SVG Bézier math with spring physics.
 * Simulates genuine physical paper stiffness, cylindrical curl highlight, and cast shadow.
 */
export function initPageCurl(selector = '.launch-card') {
  if (typeof window === 'undefined') return;
  const reduced = window.matchMedia?.('(prefers-reduced-motion: reduce)').matches;

  const cards = document.querySelectorAll(selector);

  cards.forEach((card, idx) => {
    if (card.querySelector('.page-curl-svg')) return;

    // Card needs relative positioning and overflow hidden
    card.style.position = 'relative';

    const SIZE = 84; // SVG viewport size in px
    const ID = `curl-${idx}`;

    const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    svg.setAttribute('class', 'page-curl-svg');
    svg.setAttribute('viewBox', `0 0 ${SIZE} ${SIZE}`);
    svg.setAttribute('aria-hidden', 'true');
    svg.style.cssText = `
      position: absolute;
      top: 0;
      right: 0;
      width: ${SIZE}px;
      height: ${SIZE}px;
      pointer-events: none;
      z-index: 10;
      overflow: visible;
    `;

    svg.innerHTML = `
      <defs>
        <!-- Exposed void lining (underneath paper) -->
        <linearGradient id="${ID}-under" x1="0%" y1="100%" x2="100%" y2="0%">
          <stop offset="0%" stop-color="#b5ff38" />
          <stop offset="65%" stop-color="#98e61e" />
          <stop offset="100%" stop-color="#6bb800" />
        </linearGradient>

        <!-- Curled flap cylindrical specular highlight (paper cylinder) -->
        <linearGradient id="${ID}-flap" x1="0%" y1="100%" x2="100%" y2="0%">
          <stop offset="0%" stop-color="#fdfef9" />
          <stop offset="42%" stop-color="#ffffff" />
          <stop offset="55%" stop-color="#e2e6d5" />
          <stop offset="78%" stop-color="#f4f6ee" />
          <stop offset="100%" stop-color="#ffffff" />
        </linearGradient>

        <!-- Soft shadow cast by the curl onto the card -->
        <linearGradient id="${ID}-shadow" x1="100%" y1="0%" x2="0%" y2="100%">
          <stop offset="0%" stop-color="rgba(18, 22, 11, 0.45)" />
          <stop offset="45%" stop-color="rgba(18, 22, 11, 0.22)" />
          <stop offset="100%" stop-color="rgba(18, 22, 11, 0)" />
        </linearGradient>

        <filter id="${ID}-blur" x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur stdDeviation="2.5" />
        </filter>
      </defs>

      <!-- 1. Exposed under-layer triangle (neon green lining) -->
      <path class="curl-under" fill="url(#${ID}-under)" stroke="#12160b" stroke-width="1.8" />

      <!-- 2. Cast shadow under the curled flap -->
      <path class="curl-shadow" fill="url(#${ID}-shadow)" filter="url(#${ID}-blur)" opacity="0.8" />

      <!-- 3. Curled paper flap with Bézier cylindrical curvature -->
      <path class="curl-flap" fill="url(#${ID}-flap)" stroke="#12160b" stroke-width="1.8" stroke-linejoin="round" />
    `;

    card.appendChild(svg);

    const underPath = svg.querySelector('.curl-under');
    const shadowPath = svg.querySelector('.curl-shadow');
    const flapPath = svg.querySelector('.curl-flap');

    // State for spring physics
    let curSize = 20;    // resting idle size in px
    let targetSize = 20; // target size
    let vel = 0;
    let rafId = 0;

    function renderGeometry(s) {
      if (s < 1) {
        underPath.setAttribute('d', '');
        shadowPath.setAttribute('d', '');
        flapPath.setAttribute('d', '');
        return;
      }

      const W = SIZE;
      // Corner is at (W, 0)
      const topX = W - s;
      const topY = 0;
      const rightX = W;
      const rightY = s;

      // 1. Under-layer void: triangle (topX, 0) -> (W, 0) -> (W, rightY)
      underPath.setAttribute('d', `M ${topX} 0 L ${W} 0 L ${W} ${rightY} Z`);

      // 2. Curled tip position (reflects with physical lift & curl curve)
      // Original corner (W, 0) curls inward past the diagonal
      const curlLead = s * 0.96;
      const tipX = W - curlLead;
      const tipY = curlLead * 0.94;

      // Bézier control points to produce the cylindrical paper curvature
      // Instead of flat straight lines, control points bow outwards slightly
      const cp1X = topX + s * 0.18;
      const cp1Y = tipY * 0.38;

      const cp2X = tipX * 0.98 + s * 0.35;
      const cp2Y = rightY - s * 0.18;

      // Flap path: from (topX, 0) curve to (tipX, tipY), then curve to (W, rightY), then close along fold
      const flapD = `M ${topX} 0 Q ${cp1X} ${cp1Y} ${tipX} ${tipY} Q ${cp2X} ${cp2Y} ${rightX} ${rightY} Z`;
      flapPath.setAttribute('d', flapD);

      // 3. Shadow along the fold line
      const shadowSpread = Math.min(16, s * 0.32);
      const shTopX = topX - shadowSpread;
      const shRightY = rightY + shadowSpread;
      const shadowD = `M ${topX} 0 L ${rightX} ${rightY} L ${rightX - 3} ${shRightY} L ${shTopX} 3 Z`;
      shadowPath.setAttribute('d', shadowD);
    }

    renderGeometry(curSize);

    function step() {
      rafId = 0;
      // Spring physics (stiffness: 190, damping: 15)
      const force = 190 * (targetSize - curSize) - 15 * vel;
      vel += force * (1 / 60);
      curSize += vel * (1 / 60);

      renderGeometry(curSize);

      if (Math.abs(targetSize - curSize) > 0.1 || Math.abs(vel) > 0.1) {
        rafId = requestAnimationFrame(step);
      } else {
        curSize = targetSize;
        vel = 0;
        renderGeometry(curSize);
      }
    }

    function wake(size) {
      targetSize = size;
      if (!rafId && !reduced) {
        rafId = requestAnimationFrame(step);
      } else if (reduced) {
        curSize = size;
        renderGeometry(size);
      }
    }

    card.addEventListener('pointerenter', () => {
      // On hover: peel back smoothly to 68px
      wake(68);
    });

    card.addEventListener('pointerleave', () => {
      // On leave: smoothly settle back to resting micro-curl (20px)
      wake(20);
    });
  });
}
