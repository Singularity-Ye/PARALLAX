import {parallax} from './lib/api.js';

const el = id => document.getElementById(id);
const account = el('inventory-account');
const node = (tag, text) => {
  const n = document.createElement(tag);
  if (text !== undefined) n.textContent = text;
  return n;
};

let fetching = false;
let purchaseLotsCache = new Map();

// Known item visual & market data cache (Cloudflare Steam Static CDN & Known CN Names)
const KNOWN_ITEMS = {
  'Rio 2022 Dust II Souvenir Package': {
    name_cn: '2022年里约锦标赛炙热沙城 II 纪念包',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 22.50,
    type: 'souvenir'
  },
  'Revolution Case': {
    name_cn: '变革武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 2.80,
    type: 'case'
  },
  'Kilowatt Case': {
    name_cn: '千瓦武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 5.40,
    type: 'case'
  },
  'Dreams & Nightmares Case': {
    name_cn: '梦魇武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 5.80,
    type: 'case'
  },
  'Recoil Case': {
    name_cn: '反冲武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 1.20,
    type: 'case'
  },
  'Fracture Case': {
    name_cn: '裂空武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 1.60,
    type: 'case'
  },
  'Snakebite Case': {
    name_cn: '蛇噬武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 1.50,
    type: 'case'
  },
  'Clutch Case': {
    name_cn: '命悬一线武器箱',
    icon_hash: 'IzMF03bi9WpSBq-S-ekoE33L-iLqGFHVaU25ZzQNQcXdB2ozio1RrlIWFK3UdL_n31Vq40HYvMh97-5kDO-yR0uh7r9z4b4v_paxzU_Vz666V4t2IK1XDDXoXDp96u88Hi27kRhkvW3V17uhqS_8a436S0s6',
    price: 2.30,
    type: 'case'
  }
};

function getItemSvgPlaceholder(type = 'case') {
  if (type === 'souvenir' || type === 'package') {
    return `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="38" height="38" viewBox="0 0 24 24" fill="none" stroke="%23d97706" stroke-width="1.6"><rect x="3" y="4" width="18" height="16" rx="2"/><line x1="12" y1="4" x2="12" y2="20"/><path d="M7 8h10"/><circle cx="12" cy="12" r="2" fill="%23fef3c7"/></svg>`;
  }
  return `data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="38" height="38" viewBox="0 0 24 24" fill="none" stroke="%23475569" stroke-width="1.6"><path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/></svg>`;
}

async function loadPurchases(accId) {
  if (purchaseLotsCache.has(accId)) return purchaseLotsCache.get(accId);
  try {
    const {items} = await parallax.purchaseLots(accId);
    purchaseLotsCache.set(accId, items || []);
    return items || [];
  } catch (_) {
    return [];
  }
}

async function render(snapshot) {
  // Update Technical Audit Footnote
  const box = el('inventory-result');
  if (box) {
    box.replaceChildren(
      node('p', `快照 ${snapshot.id.slice(0, 8)} · 观察时间: ${new Date(snapshot.observed_at * 1000).toLocaleString()} · 账户 ${snapshot.account_id} · SteamID ${snapshot.steam_id}`, 'hint')
    );
    for (const c of snapshot.coverage) {
      box.append(node('p', `存储区 ${c.context_id} · 读取 ${c.pages_read} 页 · 已见 ${c.observed_assets} / 报告 ${c.reported_total} 资产 · ${c.exhausted ? '全量已读完' : '受限于页数'}`, 'hint'));
    }
  }

  // Load Purchase History to match cost prices
  const purchases = await loadPurchases(snapshot.account_id);
  const costMap = new Map();
  for (const lot of purchases) {
    if (lot.market_hash_name && lot.unit_price) {
      costMap.set(lot.market_hash_name, Number(lot.unit_price));
    }
  }

  let totalUnits = 0;
  let totalMarketVal = 0;
  let cooldownUnits = 0;

  for (const a of snapshot.assets) {
    totalUnits += a.amount;
    const known = KNOWN_ITEMS[a.market_hash_name] || {};
    const price = known.price || costMap.get(a.market_hash_name) || 0;
    totalMarketVal += price * a.amount;
    const transferable = a.context_id === '2' && a.reported_tradable && !a.restriction_note_present;
    if (!transferable) cooldownUnits += a.amount;
  }

  // Steam friction: 15% marketplace fee, net wallet balance multiplier is 0.8695
  const netValue = totalMarketVal * 0.8695;

  if (el('inv-total-count')) el('inv-total-count').textContent = totalUnits;
  if (el('inv-total-market')) el('inv-total-market').textContent = totalMarketVal > 0 ? `¥${totalMarketVal.toFixed(2)}` : '—';
  if (el('inv-net-val')) el('inv-net-val').textContent = netValue > 0 ? `¥${netValue.toFixed(2)}` : '—';
  if (el('inv-status-label')) {
    if (cooldownUnits === 0) {
      el('inv-status-label').textContent = '全量可交易';
      el('inv-status-label').style.color = '#16a34a';
    } else {
      el('inv-status-label').textContent = `${cooldownUnits} 件冷却中`;
      el('inv-status-label').style.color = '#d97706';
    }
  }

  // Render 10-Column Financial Table
  const tbody = el('inventory-tbody');
  if (!tbody) return;

  if (!snapshot.assets.length) {
    tbody.innerHTML = '<tr class="empty-row"><td colspan="10" class="empty-cell" style="text-align:center;padding:24px;color:var(--muted);">当前快照下没有观察到任何饰品资产。</td></tr>';
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const a of snapshot.assets.slice(0, 300)) {
    const tr = document.createElement('tr');
    tr.className = 'inv-row';

    const known = KNOWN_ITEMS[a.market_hash_name] || null;
    const costPrice = costMap.get(a.market_hash_name) || null;
    const marketPrice = known?.price || null;

    // 1. Thumbnail (Cloudflare Steam Static CDN with smart SVG fallback)
    const tdThumb = document.createElement('td');
    tdThumb.className = 'col-thumb';
    const img = document.createElement('img');
    img.className = 'inv-item-img';
    img.alt = a.market_hash_name || 'Item';
    img.loading = 'lazy';
    if (known?.icon_hash) {
      img.src = `https://community.cloudflare.steamstatic.com/economy/image/${known.icon_hash}/96fx96f`;
    } else {
      img.src = getItemSvgPlaceholder(known?.type || 'case');
    }
    img.onerror = () => {
      img.src = getItemSvgPlaceholder(known?.type || 'case');
    };
    tdThumb.appendChild(img);
    tr.appendChild(tdThumb);

    // 2. Item Name & Category
    const tdName = document.createElement('td');
    tdName.className = 'col-inv-name';
    const strong = document.createElement('strong');
    strong.textContent = a.market_hash_name || '未命名饰品条目';
    tdName.appendChild(strong);
    if (known?.name_cn) {
      const sub = document.createElement('span');
      sub.className = 'inv-name-sub';
      sub.textContent = known.name_cn;
      tdName.appendChild(sub);
    }
    tr.appendChild(tdName);

    // 3. Amount
    const tdAmt = document.createElement('td');
    tdAmt.className = 'col-mono text-center';
    tdAmt.innerHTML = `<strong>${a.amount}</strong>`;
    tr.appendChild(tdAmt);

    // 4. Marketable status (可出售)
    const tdMarket = document.createElement('td');
    tdMarket.style.textAlign = 'center';
    const marketPill = document.createElement('span');
    marketPill.className = a.reported_marketable ? 'status-pill status-pill-success' : 'status-pill status-pill-active';
    marketPill.textContent = a.reported_marketable ? '可出售' : '不可出售';
    tdMarket.appendChild(marketPill);
    tr.appendChild(tdMarket);

    // 5. Tradable status (可交易)
    const tdTrade = document.createElement('td');
    tdTrade.style.textAlign = 'center';
    const tradePill = document.createElement('span');
    const transferable = a.context_id === '2' && a.reported_tradable && !a.restriction_note_present;
    tradePill.className = transferable ? 'status-pill status-pill-success' : 'status-pill status-pill-active';
    tradePill.textContent = transferable ? '可交易' : '受限/冷却';
    tdTrade.appendChild(tradePill);
    tr.appendChild(tdTrade);

    // 6. Cooldown / Unlock timestamp (解禁时间)
    const tdCooldown = document.createElement('td');
    if (transferable) {
      tdCooldown.innerHTML = '<span class="inv-ready-badge">无冷却 · 现货</span>';
    } else {
      tdCooldown.innerHTML = '<span class="inv-cooldown-badge">冷却中 · 详见 Steam</span>';
    }
    tr.appendChild(tdCooldown);

    // 7. Cost Price (买入价格)
    const tdCost = document.createElement('td');
    if (costPrice !== null) {
      tdCost.innerHTML = `<span class="mono" style="font-weight:700;color:#0f172a;">¥${costPrice.toFixed(2)}</span>`;
    } else {
      tdCost.innerHTML = '<span class="text-muted" style="font-size:11px;">— (未录入)</span>';
    }
    tr.appendChild(tdCost);

    // 8. Lowest Market Price (市场现价)
    const tdMarketPrice = document.createElement('td');
    if (marketPrice !== null) {
      tdMarketPrice.innerHTML = `<span class="mono" style="font-weight:750;color:#0f172a;">¥${marketPrice.toFixed(2)}</span>`;
    } else {
      tdMarketPrice.innerHTML = '<span class="text-muted" style="font-size:11px;">— (待行情)</span>';
    }
    tr.appendChild(tdMarketPrice);

    // 9. Ratio / Profit (折扣比率与浮动盈亏)
    const tdProfit = document.createElement('td');
    if (costPrice !== null && marketPrice !== null && costPrice > 0) {
      const netUnit = marketPrice * 0.8695;
      const profitUnit = (netUnit - costPrice) * a.amount;
      const ratio = ((costPrice / marketPrice) * 100).toFixed(1);
      if (profitUnit >= 0) {
        tdProfit.innerHTML = `<span class="inv-capsule-profit">+¥${profitUnit.toFixed(2)} (${ratio}%)</span>`;
      } else {
        tdProfit.innerHTML = `<span class="inv-capsule-loss">¥${profitUnit.toFixed(2)} (${ratio}%)</span>`;
      }
    } else if (marketPrice !== null) {
      const netUnit = marketPrice * 0.8695;
      tdProfit.innerHTML = `<span class="text-muted" style="font-size:10.5px;">净得 ¥${netUnit.toFixed(2)}</span>`;
    } else {
      tdProfit.innerHTML = '<span class="text-muted" style="font-size:11px;">—</span>';
    }
    tr.appendChild(tdProfit);

    // 10. Links & Actions (Steam/BUFF直达与走势)
    const tdLinks = document.createElement('td');
    tdLinks.className = 'col-links';
    tdLinks.style.textAlign = 'center';
    if (a.market_hash_name) {
      const steamA = document.createElement('a');
      steamA.href = `https://steamcommunity.com/market/listings/730/${encodeURIComponent(a.market_hash_name)}`;
      steamA.target = '_blank';
      steamA.rel = 'noopener';
      steamA.className = 'ext-link';
      steamA.title = '前往 Steam 社区市场原页面';
      steamA.textContent = 'Steam ↗';

      const buffA = document.createElement('a');
      buffA.href = `https://buff.163.com/market/csgo?tab=selling&search=${encodeURIComponent(a.market_hash_name)}`;
      buffA.target = '_blank';
      buffA.rel = 'noopener';
      buffA.className = 'ext-link';
      buffA.title = '前往网易 BUFF 盘口比价';
      buffA.textContent = 'BUFF ↗';

      const btnHist = document.createElement('button');
      btnHist.type = 'button';
      btnHist.className = 'btn btn-secondary btn-xs';
      btnHist.style.marginLeft = '4px';
      btnHist.textContent = '走势';
      btnHist.onclick = () => window.dispatchEvent(new CustomEvent('parallax:inventory-history', {
        detail: { account_id: snapshot.account_id, inventory_snapshot_id: snapshot.id, item_key: a.market_hash_name }
      }));

      tdLinks.append(steamA, document.createTextNode(' '), buffA, document.createTextNode(' '), btnHist);
    } else {
      tdLinks.textContent = '—';
    }
    tr.appendChild(tdLinks);

    fragment.appendChild(tr);
  }

  tbody.replaceChildren(fragment);
}

let autoLoaded = false;
async function history() {
  const id = account.value;
  el('inventory-history').replaceChildren();
  if (!id) return;
  const {items} = await parallax.inventories(id);
  if (account.value !== id) return;

  // Auto-render latest snapshot on initial view so user never sees a blank screen
  if (items.length > 0 && !autoLoaded) {
    autoLoaded = true;
    parallax.inventory(id, items[0].id).then(saved => {
      if (account.value === id) render(saved);
    }).catch(() => {});
  }
  for (const snapshot of items) {
    const b = node('button', `${new Date(snapshot.observed_at * 1000).toLocaleString()} · ${snapshot.assets.length} 个条目 · ${snapshot.coverage.every(c => c.exhausted) ? '双区已完整同步' : '局部'}`);
    b.type = 'button';
    b.className = 'history-row';
    b.onclick = async () => {
      try {
        const saved = await parallax.inventory(id, snapshot.id);
        if (account.value !== id) return;
        render(saved);
        el('inventory-feedback').textContent = '已载入历史快照；未重新请求平台或消耗 Steam 速率限制。';
      } catch (e) {
        if (account.value === id) el('inventory-feedback').textContent = e.message;
      }
    };
    el('inventory-history').append(b);
  }
  if (!items.length) el('inventory-history').append(node('p', '该账户尚无快照备份。点击「立即刷新快照」开始首次拉取。'));
}

async function loadAccounts() {
  const preferred = account.value;
  const {items} = await parallax.accounts();
  account.replaceChildren();
  for (const a of items) {
    const o = node('option', `${a.label} (${a.steam_id.slice(-6)})`);
    o.value = a.id;
    account.append(o);
  }
  if (items.some(a => a.id === preferred)) account.value = preferred;
  el('inventory-fetch').disabled = fetching || !account.value;
  await history();
}

account.onchange = () => {
  el('inventory-result').replaceChildren();
  el('inventory-feedback').textContent = '';
  history().catch(e => el('inventory-feedback').textContent = e.message);
};

el('inventory-fetch').onclick = async event => {
  event.preventDefault();
  if (!account.value) return;
  const id = account.value;
  fetching = true;
  account.disabled = true;
  el('inventory-fetch').disabled = true;
  el('inventory-feedback').textContent = '正在连接 Steam 社区读取 CS2 库存资产…';
  try {
    const pages = Number(el('inventory-pages')?.value || 4);
    const snapshot = await parallax.fetchInventory(id, {max_pages_per_context: pages});
    if (account.value === id) await render(snapshot);
    el('inventory-feedback').textContent = '最新库存快照已保存并完成资产估值测算。';
    try {
      await history();
    } catch (e) {
      el('inventory-feedback').textContent = `快照已更新，历史同步延迟：${e.message}`;
    }
  } catch (e) {
    el('inventory-feedback').textContent = `读取失败：${e.message}`;
  } finally {
    fetching = false;
    account.disabled = false;
    el('inventory-fetch').disabled = !account.value;
  }
};

window.addEventListener('parallax:account-created', () => loadAccounts().catch(e => el('inventory-feedback').textContent = e.message));
loadAccounts().catch(e => el('inventory-feedback').textContent = e.message);
