import {motionScope,springStep,frameClock} from './motion.js';
/**
 * Vanilla SVG Bézier Page Curl (柔韧书页矢量弧面卷角 - 黄金克制版)
 * Zero external libraries. 100% vector math with self-sleeping spring physics.
 * Stops requesting frames once the spring settles.
 */
export function initPageCurl(selector = '.launch-card, [data-page-curl]') {
  if (typeof window === 'undefined') return;
  const scope=motionScope();

  const cards = document.querySelectorAll(selector);

  cards.forEach((card, idx) => {
    if (card.querySelector('.page-curl-svg')) return;

    // Card needs relative positioning
    const previousPosition=card.style.position;
    card.style.position = 'relative';

    const SIZE = 72; // SVG viewport size in px
    const ID = `curl-${idx}`;

    // Calibrated golden parameters: elegant, restrained, non-intrusive
    const IDLE_SIZE = 14;  // Resting micro-lift
    const HOVER_SIZE = 46; // Restrained hover peel (leaves plenty of breathing room for text/arrows)

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
          <stop offset="0%" stop-color="#fdfdf9" />
          <stop offset="65%" stop-color="#f3f5ec" />
          <stop offset="100%" stop-color="#dce1d3" />
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
          <stop offset="0%" stop-color="rgba(18, 22, 11, 0.40)" />
          <stop offset="45%" stop-color="rgba(18, 22, 11, 0.18)" />
          <stop offset="100%" stop-color="rgba(18, 22, 11, 0)" />
        </linearGradient>

        <filter id="${ID}-blur" x="-30%" y="-30%" width="160%" height="160%">
          <feGaussianBlur stdDeviation="2" />
        </filter>
      </defs>

      <!-- 1. Exposed under-layer triangle (neon green lining) -->
      <path class="curl-under" fill="url(#${ID}-under)" stroke="#12160b" stroke-width="1.6" />

      <!-- 2. Cast shadow under the curled flap -->
      <path class="curl-shadow" fill="url(#${ID}-shadow)" filter="url(#${ID}-blur)" opacity="0.75" />

      <!-- 3. Curled paper flap with Bézier cylindrical curvature -->
      <path class="curl-flap" fill="url(#${ID}-flap)" stroke="#12160b" stroke-width="1.6" stroke-linejoin="round" />
    `;

    card.appendChild(svg);

    const underPath = svg.querySelector('.curl-under');
    const shadowPath = svg.querySelector('.curl-shadow');
    const flapPath = svg.querySelector('.curl-flap');

    // State for spring physics
    let curSize = IDLE_SIZE;
    let targetSize = IDLE_SIZE;
    const state={x:IDLE_SIZE,v:0};
    let clock=frameClock();
    let rafId = 0;

    function renderGeometry(s) {
      if (s < 1) {
        underPath.setAttribute('d', '');
        shadowPath.setAttribute('d', '');
        flapPath.setAttribute('d', '');
        return;
      }

      const W = SIZE;
      const topX = W - s;
      const rightY = s;

      // 1. Under-layer void: triangle (topX, 0) -> (W, 0) -> (W, rightY)
      underPath.setAttribute('d', `M ${topX} 0 L ${W} 0 L ${W} ${rightY} Z`);

      // 2. Curled tip position (reflects with physical lift & curl curve)
      const curlLead = s * 0.95;
      const tipX = W - curlLead;
      const tipY = curlLead * 0.93;

      // Bézier control points to produce the cylindrical paper curvature
      const cp1X = topX + s * 0.20;
      const cp1Y = tipY * 0.40;

      const cp2X = tipX * 0.98 + s * 0.36;
      const cp2Y = rightY - s * 0.20;

      // Flap path: from (topX, 0) curve to (tipX, tipY), then curve to (W, rightY), then close along fold
      const flapD = `M ${topX} 0 Q ${cp1X} ${cp1Y} ${tipX} ${tipY} Q ${cp2X} ${cp2Y} ${W} ${rightY} Z`;
      flapPath.setAttribute('d', flapD);

      // 3. Shadow along the fold line
      const shadowSpread = Math.min(12, s * 0.28);
      const shTopX = topX - shadowSpread;
      const shRightY = rightY + shadowSpread;
      const shadowD = `M ${topX} 0 L ${W} ${rightY} L ${W - 2} ${shRightY} L ${shTopX} 2 Z`;
      shadowPath.setAttribute('d', shadowD);
    }

    renderGeometry(curSize);

    function step(now) {
      rafId = 0;
      // Spring physics (stiffness: 220, damping: 18 - crisp, tactile paper response)
      const moving=springStep(state,targetSize,clock(now),220,18);
      curSize=state.x;renderGeometry(curSize);
      if(moving)rafId=scope.frame(step);
    }
    function wake(size) {
      targetSize=size;
      if(!rafId){clock=frameClock();rafId=scope.frame(step);}
    }
    scope.cleanup(()=>{svg.remove();card.style.position=previousPosition;});

    scope.on(card,'pointerenter', () => {
      wake(HOVER_SIZE);
    });

    scope.on(card,'pointerleave', () => {
      wake(IDLE_SIZE);
    });
  });
  return ()=>scope.dispose();
}
