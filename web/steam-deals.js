import { parallax } from './lib/api.js';
import { node } from './lib/pricing-view.js';

const el = id => document.getElementById(id);

let currentComparison = null;
let isComparing = false;

async function loadPopularGames() {
  const container = el('deals-popular-grid');
  if (!container) return;

  try {
    const res = await parallax.popularDeals();
    const games = res?.games || [];
    container.replaceChildren();

    games.forEach(g => {
      const card = node('div');
      card.className = 'route-selector-card';
      card.style.cursor = 'pointer';
      card.style.transition = 'transform 0.15s, border-color 0.15s';
      card.onmouseenter = () => { card.style.borderColor = '#48563c'; card.style.transform = 'translateY(-1px)'; };
      card.onmouseleave = () => { card.style.borderColor = '#dce1d3'; card.style.transform = ''; };

      const img = node('img');
      img.src = g.cover_image;
      img.style.width = '100%';
      img.style.borderRadius = '4px';
      img.style.marginBottom = '8px';

      const title = node('div', g.title);
      title.style.fontWeight = '600';
      title.style.fontSize = '12px';
      title.style.color = '#262b21';
      title.style.lineHeight = '1.4';

      const tag = node('span', g.tag);
      tag.className = 'badge ok';
      tag.style.marginTop = '6px';
      tag.style.display = 'inline-block';

      card.append(img, title, tag);
      card.onclick = () => {
        const input = el('deals-appid-input');
        if (input) input.value = g.appid;
        triggerCompare(g.appid);
      };

      container.append(card);
    });
  } catch (err) {
    console.warn('Failed to load popular games:', err);
  }
}

async function triggerCompare(appid) {
  if (isComparing) return;
  const statusEl = el('deals-status-msg');
  const btn = el('btn-deals-compare');
  const appidInput = el('deals-appid-input');
  const targetId = appid || Number(appidInput?.value || 0);

  if (!targetId || targetId <= 0) {
    if (statusEl) statusEl.textContent = '请先输入有效的 Steam AppID 或点击推荐游戏';
    return;
  }

  isComparing = true;
  if (btn) btn.disabled = true;
  if (statusEl) statusEl.textContent = `正在并发探测 AppID ${targetId} 的 16 区官方报价与汇率换算…`;

  try {
    const res = await parallax.compareDeals(targetId);
    currentComparison = res;
    renderComparison();
    if (statusEl) statusEl.textContent = `已完成 ${res.title} 的 16 区价格比对`;
  } catch (err) {
    if (statusEl) statusEl.textContent = `比价失败：${err.message}`;
  } finally {
    isComparing = false;
    if (btn) btn.disabled = false;
  }
}

function renderComparison() {
  if (!currentComparison) return;
  const c = currentComparison;

  // 1. Overview Card
  const overviewCard = el('deals-overview-card');
  if (overviewCard) overviewCard.style.display = 'flex';

  const coverImg = el('deals-cover-img');
  if (coverImg) coverImg.src = c.header_image;

  const gameTitle = el('deals-game-title');
  if (gameTitle) gameTitle.textContent = c.title;

  const cnPrice = el('deals-cn-price');
  if (cnPrice) {
    cnPrice.textContent = c.cn_cny_cents !== null && c.cn_cny_cents !== undefined
      ? `¥${(c.cn_cny_cents / 100).toFixed(2)}`
      : '未开售 / 免费';
  }

  const lowestRegion = el('deals-lowest-region');
  if (lowestRegion) {
    lowestRegion.textContent = c.lowest_region || '—';
  }

  const lowestPrice = el('deals-lowest-price');
  if (lowestPrice) {
    lowestPrice.textContent = c.lowest_cny_cents !== null && c.lowest_cny_cents !== undefined
      ? `¥${(c.lowest_cny_cents / 100).toFixed(2)}`
      : '—';
  }

  const savingsBadge = el('deals-savings-badge');
  if (savingsBadge) {
    if (c.max_savings_pct && c.max_savings_pct > 0) {
      savingsBadge.style.display = 'inline-block';
      savingsBadge.textContent = `立省 ${c.max_savings_pct}%`;
    } else {
      savingsBadge.style.display = 'none';
    }
  }

  // 2. 16 Regions Table
  const tbody = el('deals-regions-tbody');
  if (!tbody) return;
  tbody.replaceChildren();

  const regions = c.regions || [];
  regions.forEach(r => {
    const tr = node('tr');

    // Region code + Name
    const tdReg = node('td');
    const nameDiv = node('div', r.region_name);
    nameDiv.style.fontWeight = '600';
    nameDiv.style.color = '#262b21';
    const codeSpan = node('span', `区码: ${r.region_code.toUpperCase()} · 币种: ${r.currency}`);
    codeSpan.className = 'route-desc-hint';
    tdReg.append(nameDiv, codeSpan);

    // Native formatted price
    const tdNative = node('td', r.formatted_price);
    tdNative.style.fontWeight = '500';

    // Discount pct
    const tdDisc = node('td');
    if (r.discount_pct > 0) {
      const b = node('span', `-${r.discount_pct}%`);
      b.className = 'badge ok';
      tdDisc.append(b);
    } else {
      tdDisc.textContent = '无折扣';
      tdDisc.style.color = '#6b7263';
      tdDisc.style.fontSize = '12px';
    }

    // CNY equivalent
    const tdCny = node('td');
    if (r.cny_cents !== null && r.cny_cents !== undefined) {
      tdCny.textContent = `¥${(r.cny_cents / 100).toFixed(2)}`;
      tdCny.style.fontWeight = '700';
      tdCny.style.color = '#262b21';
    } else {
      tdCny.textContent = '—';
    }

    // Diff vs CN
    const tdDiff = node('td');
    if (r.region_code === 'cn') {
      tdDiff.textContent = '基准国区';
      tdDiff.style.color = '#6b7263';
      tdDiff.style.fontSize = '12px';
    } else if (r.cny_diff_cents !== null && r.cny_diff_cents !== undefined) {
      const diffVal = r.cny_diff_cents / 100;
      if (diffVal > 0) {
        tdDiff.textContent = `省 ¥${diffVal.toFixed(2)}`;
        tdDiff.style.color = '#355e3b';
        tdDiff.style.fontWeight = '600';
      } else if (diffVal < 0) {
        tdDiff.textContent = `贵 ¥${Math.abs(diffVal).toFixed(2)}`;
        tdDiff.style.color = '#a3433d';
      } else {
        tdDiff.textContent = '持平';
        tdDiff.style.color = '#6b7263';
      }
    } else {
      tdDiff.textContent = '—';
    }

    // Value ratio
    const tdRatio = node('td');
    if (r.value_ratio && r.value_ratio > 0) {
      const ratioBadge = node('span', `+${r.value_ratio}x 价值比`);
      ratioBadge.className = 'badge ok';
      tdRatio.append(ratioBadge);
    } else {
      tdRatio.textContent = '—';
    }

    // Lowest status tag
    const tdStatus = node('td');
    tdStatus.style.textAlign = 'right';
    if (c.lowest_region === r.region_name && r.region_code !== 'cn') {
      const star = node('span', '最低价区');
      star.className = 'badge warn';
      tdStatus.append(star);
    } else if (r.region_code === 'cn') {
      const cnTag = node('span', '国区');
      cnTag.className = 'badge muted';
      tdStatus.append(cnTag);
    }

    tr.append(tdReg, tdNative, tdDisc, tdCny, tdDiff, tdRatio, tdStatus);
    tbody.append(tr);
  });
}

function initDeals() {
  const btn = el('btn-deals-compare');
  if (btn) btn.onclick = () => triggerCompare();

  const input = el('deals-appid-input');
  if (input) {
    input.onkeydown = e => {
      if (e.key === 'Enter') triggerCompare();
    };
  }

  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'steam-deals') {
      loadPopularGames();
    }
  });

  if (location.hash === '#steam-deals') {
    loadPopularGames();
  }
}

initDeals();
loadPopularGames().catch(console.error);
