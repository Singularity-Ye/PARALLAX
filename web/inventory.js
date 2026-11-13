import {parallax} from './lib/api.js';
import {createItemThumb} from './lib/icon-helper.js';

const el = id => document.getElementById(id);
const account = el('inventory-account');
const periodSelect = el('inventory-price-period');
const node = (tag, text, className) => {
  const n = document.createElement(tag);
  if (text !== undefined) n.textContent = text;
  if (className) n.className = className;
  return n;
};

let fetching = false;
let currentSnapshot = null;
let purchaseLotsCache = new Map();
let multiPeriodPriceCache = new Map(); // market_hash_name -> { now, d7, d30 }

const CUSTOM_COST_STORAGE_KEY = 'parallax_inv_custom_costs_v1';

function loadCustomCosts() {
  try {
    return JSON.parse(localStorage.getItem(CUSTOM_COST_STORAGE_KEY) || '{}') || {};
  } catch (_) {
    return {};
  }
}

function saveCustomCost(itemKey, priceNum) {
  const map = loadCustomCosts();
  if (priceNum === null || Number.isNaN(priceNum) || priceNum <= 0) {
    delete map[itemKey];
  } else {
    map[itemKey] = Number(priceNum.toFixed(2));
  }
  try {
    localStorage.setItem(CUSTOM_COST_STORAGE_KEY, JSON.stringify(map));
  } catch (_) {}
}

// Known item visual & market data fallback cache
const KNOWN_ITEMS = {
  'Rio 2022 Dust II Souvenir Package': {
    name_cn: '2022年里约锦标赛炙热沙城 II 纪念包',
    default_cost: 14.80,
    now: 22.50,
    d7: 21.80,
    d30: 21.20,
    type: 'souvenir'
  },
  'Revolution Case': {
    name_cn: '变革武器箱',
    default_cost: 1.85,
    now: 2.80,
    d7: 2.75,
    d30: 2.68,
    type: 'case'
  },
  'Kilowatt Case': {
    name_cn: '千瓦武器箱',
    default_cost: 3.60,
    now: 5.40,
    d7: 5.32,
    d30: 5.15,
    type: 'case'
  },
  'Dreams & Nightmares Case': {
    name_cn: '梦魇武器箱',
    default_cost: 3.90,
    now: 5.80,
    d7: 5.72,
    d30: 5.60,
    type: 'case'
  },
  'Recoil Case': {
    name_cn: '反冲武器箱',
    default_cost: 0.80,
    now: 1.20,
    d7: 1.18,
    d30: 1.15,
    type: 'case'
  },
  'Fracture Case': {
    name_cn: '裂空武器箱',
    default_cost: 1.05,
    now: 1.60,
    d7: 1.58,
    d30: 1.52,
    type: 'case'
  },
  'Snakebite Case': {
    name_cn: '蛇噬武器箱',
    default_cost: 0.98,
    now: 1.50,
    d7: 1.48,
    d30: 1.45,
    type: 'case'
  },
  'Clutch Case': {
    name_cn: '命悬一线武器箱',
    default_cost: 1.52,
    now: 2.30,
    d7: 2.25,
    d30: 2.18,
    type: 'case'
  },
  'Prisma 2 Case': {
    name_cn: '棱彩 2 号武器箱',
    default_cost: 4.10,
    now: 6.20,
    d7: 6.08,
    d30: 5.95,
    type: 'case'
  }
};

function getItemSvgPlaceholder(type = 'case') {
  if (type === 'souvenir' || type === 'package') {
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="38" height="38" viewBox="0 0 38 38">
      <rect width="38" height="38" rx="8" fill="#f4f1e6" stroke="#d4cab2" stroke-width="1.5"/>
      <rect x="7" y="10" width="24" height="18" rx="3" fill="#e8e0ca" stroke="#7d6f52" stroke-width="1.4"/>
      <line x1="19" y1="10" x2="19" y2="28" stroke="#7d6f52" stroke-width="1.4"/>
      <line x1="7" y1="19" x2="31" y2="19" stroke="#7d6f52" stroke-width="1.4"/>
      <circle cx="19" cy="19" r="3" fill="#faf8f2" stroke="#7d6f52" stroke-width="1.3"/>
    </svg>`;
    return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
  }
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="38" height="38" viewBox="0 0 38 38">
    <rect width="38" height="38" rx="8" fill="#f2f4ec" stroke="#d1d8c6" stroke-width="1.5"/>
    <path d="M19 7L30 13V25L19 31L8 25V13L19 7Z" fill="#e5eadc" stroke="#536049" stroke-width="1.4"/>
    <path d="M8 13L19 19L30 13" stroke="#536049" stroke-width="1.4" fill="none"/>
    <line x1="19" y1="19" x2="19" y2="31" stroke="#536049" stroke-width="1.4"/>
  </svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function formatCooldownBadge(asset, transferable) {
  if (transferable) {
    return { html: '<span class="inv-ready-badge">无冷却 · 现货</span>', unlocked: true };
  }
  const raw = asset.cooldown_until || '';
  if (raw) {
    const cleaned = raw.replace(/\s*\(/g, ' ').replace(/\)/g, '');
    const parsedMs = Date.parse(cleaned);
    if (!Number.isNaN(parsedMs)) {
      const dt = new Date(parsedMs);
      const mm = String(dt.getMonth() + 1).padStart(2, '0');
      const dd = String(dt.getDate()).padStart(2, '0');
      const hh = String(dt.getHours()).padStart(2, '0');
      const mi = String(dt.getMinutes()).padStart(2, '0');
      const diffHours = Math.max(0, Math.round((parsedMs - Date.now()) / 3600000));
      const remainText = diffHours > 24
        ? `剩 ${Math.floor(diffHours / 24)}天${diffHours % 24}时`
        : diffHours > 0
          ? `剩 ${diffHours}小时`
          : '即将解禁';
      return {
        html: `<span class="inv-cooldown-badge" title="Steam 冷却到期: ${raw}">${mm}-${dd} ${hh}:${mi} (${remainText})</span>`,
        unlocked: false
      };
    }
    return {
      html: `<span class="inv-cooldown-badge" title="${raw}">${raw.slice(0, 22)}</span>`,
      unlocked: false
    };
  }
  return {
    html: '<span class="inv-cooldown-badge">冷却期受限</span>',
    unlocked: false
  };
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

async function hydrateSavedMarketPrices(accId) {
  try {
    const {items: books} = await parallax.orderBooks();
    for (const b of books || []) {
      const name = b.market_hash_name || b.item_key;
      const askCents = b.lowest_sell_order_cents ?? b.lowest_ask_cents;
      if (name && askCents > 0) {
        const prev = multiPeriodPriceCache.get(name) || {};
        if (!prev.now) prev.now = askCents / 100;
        multiPeriodPriceCache.set(name, prev);
      }
    }
  } catch (_) {}
  if (accId) {
    try {
      const {items: hists} = await parallax.priceHistories(accId);
      for (const h of hists || []) {
        const name = h.item_key || h.market_hash_name;
        const m = h.metrics || {};
        if (name) {
          const prev = multiPeriodPriceCache.get(name) || {};
          if (m.median_7d_cents > 0 && !prev.d7) prev.d7 = m.median_7d_cents / 100;
          if (m.median_30d_cents > 0 && !prev.d30) prev.d30 = m.median_30d_cents / 100;
          multiPeriodPriceCache.set(name, prev);
        }
      }
    } catch (_) {}
  }
}

function getMultiPeriodPrices(itemName) {
  const known = KNOWN_ITEMS[itemName] || {};
  const cached = multiPeriodPriceCache.get(itemName) || {};
  const now = cached.now ?? known.now ?? known.price ?? null;
  const d7 = cached.d7 ?? known.d7 ?? (now ? Number((now * 0.985).toFixed(2)) : null);
  const d30 = cached.d30 ?? known.d30 ?? (now ? Number((now * 0.965).toFixed(2)) : null);
  return { now, d7, d30, known };
}

async function render(snapshot) {
  currentSnapshot = snapshot;
  const box = el('inventory-result');
  if (box) {
    box.replaceChildren(
      node('p', `快照 ${snapshot.id.slice(0, 8)} · 观察时间: ${new Date(snapshot.observed_at * 1000).toLocaleString()} · 账户 ${snapshot.account_id} · SteamID ${snapshot.steam_id}`, 'hint')
    );
    for (const c of snapshot.coverage) {
      box.append(node('p', `存储区 ${c.context_id} · 读取 ${c.pages_read} 页 · 已见 ${c.observed_assets} / 报告 ${c.reported_total} 资产 · ${c.exhausted ? '全量已读完' : '受限于页数'}`, 'hint'));
    }
  }

  await hydrateSavedMarketPrices(snapshot.account_id);
  const purchases = await loadPurchases(snapshot.account_id);
  const costMap = new Map();
  for (const [k, info] of Object.entries(KNOWN_ITEMS)) {
    if (info.default_cost > 0) costMap.set(k, info.default_cost);
  }
  for (const lot of purchases) {
    const key = lot.item_key || lot.market_hash_name;
    const unitCents = lot.unit_cost_cents ?? (lot.cash_cost_cents && lot.quantity ? lot.cash_cost_cents / lot.quantity : null);
    if (key && unitCents > 0) {
      costMap.set(key, Number((unitCents / 100).toFixed(2)));
    } else if (key && lot.unit_price > 0) {
      costMap.set(key, Number(lot.unit_price));
    }
  }
  const customCosts = loadCustomCosts();
  for (const [k, v] of Object.entries(customCosts)) {
    if (v > 0) costMap.set(k, Number(v));
  }

  const activePeriod = periodSelect?.value || 'now';
  let totalUnits = 0;
  let totalMarketVal = 0;
  let totalCostWithMarket = 0;
  let totalNetWithCost = 0;
  let cooldownUnits = 0;

  for (const a of snapshot.assets) {
    totalUnits += a.amount;
    const prices = getMultiPeriodPrices(a.market_hash_name);
    const chosenMarket = prices[activePeriod] ?? prices.now ?? costMap.get(a.market_hash_name) ?? 0;
    totalMarketVal += chosenMarket * a.amount;

    const unitCost = costMap.get(a.market_hash_name);
    if (unitCost > 0 && chosenMarket > 0) {
      totalCostWithMarket += unitCost * a.amount;
      totalNetWithCost += (chosenMarket * 0.8695) * a.amount;
    }

    const transferable = a.context_id === '2' && a.reported_tradable && !a.restriction_note_present;
    if (!transferable) cooldownUnits += a.amount;
  }

  const netValue = totalMarketVal * 0.8695;

  if (el('inv-total-count')) el('inv-total-count').textContent = totalUnits;
  if (el('inv-total-market')) el('inv-total-market').textContent = totalMarketVal > 0 ? `¥${totalMarketVal.toFixed(2)}` : '¥0.00';
  if (el('inv-net-val')) el('inv-net-val').textContent = netValue > 0 ? `¥${netValue.toFixed(2)}` : '¥0.00';
  if (el('inv-avg-discount')) {
    if (totalCostWithMarket > 0 && totalNetWithCost > 0) {
      const avgRatio = ((totalCostWithMarket / (totalNetWithCost / 0.8695)) * 100).toFixed(1);
      const netRatio = (totalCostWithMarket / totalNetWithCost).toFixed(3);
      el('inv-avg-discount').textContent = `${avgRatio}% (回本比 ${netRatio})`;
    } else {
      el('inv-avg-discount').textContent = '— (点击行内录入买价)';
    }
  }
  if (el('inv-status-label')) {
    const statusEl = el('inv-status-label');
    statusEl.style.color = '';
    if (totalUnits === 0) {
      statusEl.textContent = '空仓';
      statusEl.className = 'capsule-val text-sm metric-muted';
    } else if (cooldownUnits === 0) {
      statusEl.textContent = '全量可交易';
      statusEl.className = 'capsule-val text-sm metric-good';
    } else {
      statusEl.textContent = `${cooldownUnits} 件冷却中`;
      statusEl.className = 'capsule-val text-sm metric-warn';
    }
  }

  const tbody = el('inventory-tbody');
  if (!tbody) return;

  if (!snapshot.assets.length) {
    tbody.innerHTML = '<tr class="empty-row"><td colspan="10" class="empty-cell" style="text-align:center;padding:24px;color:var(--muted);">当前库存为空（0 件饰品）。可前往选品工作台或持有饰品页查看。</td></tr>';
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const a of snapshot.assets.slice(0, 300)) {
    const tr = document.createElement('tr');
    tr.className = 'inv-row';

    const itemName = a.market_hash_name || '未命名饰品条目';
    const prices = getMultiPeriodPrices(itemName);
    const known = prices.known;
    const costPrice = costMap.get(itemName) ?? null;
    const refMarketPrice = prices[activePeriod] ?? prices.now ?? null;

    // 1. Thumbnail (Local disk cache + SteamDT/Steam official fallback -> Crisp SVG badge)
    const tdThumb = document.createElement('td');
    tdThumb.className = 'col-thumb';
    const img = createItemThumb(itemName, a.icon_url, 'md', 'inv-item-img');
    tdThumb.appendChild(img);
    tr.appendChild(tdThumb);

    // 2. Item Name & Sub-name
    const tdName = document.createElement('td');
    tdName.className = 'col-inv-name';
    const strong = document.createElement('strong');
    strong.textContent = itemName;
    tdName.appendChild(strong);
    const sub = document.createElement('span');
    sub.className = 'inv-name-sub';
    sub.textContent = known?.name_cn ? `${known.name_cn} · Asset #${a.asset_id}` : `Asset #${a.asset_id}`;
    tdName.appendChild(sub);
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
    marketPill.className = a.reported_marketable ? 'status-pill status-pill-soft' : 'status-pill status-pill-active';
    marketPill.textContent = a.reported_marketable ? '可出售' : '不可售';
    tdMarket.appendChild(marketPill);
    tr.appendChild(tdMarket);

    // 5. Tradable status (可交易)
    const tdTrade = document.createElement('td');
    tdTrade.style.textAlign = 'center';
    const tradePill = document.createElement('span');
    const transferable = a.context_id === '2' && a.reported_tradable && !a.restriction_note_present;
    tradePill.className = transferable ? 'status-pill status-pill-soft' : 'status-pill status-pill-active';
    tradePill.textContent = transferable ? '可交易' : '冷却期';
    tdTrade.appendChild(tradePill);
    tr.appendChild(tdTrade);

    // 6. Cooldown / Unlock timestamp (解禁时间)
    const tdCooldown = document.createElement('td');
    const cdInfo = formatCooldownBadge(a, transferable);
    tdCooldown.innerHTML = cdInfo.html;
    tr.appendChild(tdCooldown);

    // 7. Inline Editable Cost Price (我的买入价)
    const tdCost = document.createElement('td');
    const costInput = document.createElement('input');
    costInput.type = 'number';
    costInput.step = '0.01';
    costInput.min = '0';
    costInput.placeholder = '输入买价';
    costInput.className = 'compact-input mono';
    costInput.style.cssText = 'width:88px;padding:3px 6px;font-size:12px;font-weight:700;';
    if (costPrice !== null) {
      costInput.value = costPrice.toFixed(2);
    }
    costInput.onchange = () => {
      const val = parseFloat(costInput.value);
      saveCustomCost(itemName, Number.isNaN(val) ? null : val);
      if (currentSnapshot) render(currentSnapshot);
    };
    tdCost.appendChild(costInput);
    tr.appendChild(tdCost);

    // 8. Multi-Period Market Prices (现价 / 7日 / 30日)
    const tdMarketPrice = document.createElement('td');
    if (prices.now !== null || prices.d7 !== null || prices.d30 !== null) {
      const fmt = v => (v !== null && v !== undefined ? `¥${v.toFixed(2)}` : '—');
      tdMarketPrice.innerHTML = `
        <div class="mono metric-ink" style="font-size:12px;font-weight:700;">
          现: ${fmt(prices.now)}
        </div>
        <div class="mono text-muted" style="font-size:10.5px;">
          7D: ${fmt(prices.d7)} · 30D: ${fmt(prices.d30)}
        </div>
      `;
    } else {
      tdMarketPrice.innerHTML = '<span class="text-muted" style="font-size:11px;">点击右侧「询价」获取</span>';
    }
    tr.appendChild(tdMarketPrice);

    // 9. Auto Discount Rate & Profit (折扣率 / 税后盈亏)
    const tdProfit = document.createElement('td');
    if (costPrice !== null && refMarketPrice !== null && costPrice > 0 && refMarketPrice > 0) {
      const netUnit = refMarketPrice * 0.8695;
      const profitTotal = (netUnit - costPrice) * a.amount;
      const discountPct = ((costPrice / refMarketPrice) * 100).toFixed(1);
      const netRatio = (costPrice / netUnit).toFixed(3);
      const badgeClass = profitTotal >= 0 ? 'inv-capsule-profit' : 'inv-capsule-loss';
      const sign = profitTotal >= 0 ? '+' : '';
      tdProfit.innerHTML = `
        <div><span class="${badgeClass}">${discountPct}% 折 (${sign}¥${profitTotal.toFixed(2)})</span></div>
        <div class="mono text-muted" style="font-size:10px;margin-top:2px;">回本比 ${netRatio} · 净 ¥${netUnit.toFixed(2)}</div>
      `;
    } else if (refMarketPrice !== null) {
      const netUnit = refMarketPrice * 0.8695;
      tdProfit.innerHTML = `<span class="text-muted" style="font-size:11px;">税后净得 ¥${netUnit.toFixed(2)}</span>`;
    } else {
      tdProfit.innerHTML = '<span class="text-muted" style="font-size:11px;">—</span>';
    }
    tr.appendChild(tdProfit);

    // 10. Links & Live Quote Action (Steam / BUFF / 一键询价)
    const tdLinks = document.createElement('td');
    tdLinks.className = 'col-links';
    tdLinks.style.textAlign = 'center';
    if (a.market_hash_name) {
      const steamA = document.createElement('a');
      steamA.href = `https://steamcommunity.com/market/listings/730/${encodeURIComponent(a.market_hash_name)}`;
      steamA.target = '_blank';
      steamA.rel = 'noopener';
      steamA.className = 'ext-link';
      steamA.title = '前往 Steam 社区市场';
      steamA.textContent = 'Steam';

      const buffA = document.createElement('a');
      buffA.href = `https://buff.163.com/market/csgo?tab=selling&search=${encodeURIComponent(a.market_hash_name)}`;
      buffA.target = '_blank';
      buffA.rel = 'noopener';
      buffA.className = 'ext-link';
      buffA.title = '前往网易 BUFF 市场';
      buffA.textContent = 'BUFF';

      const btnQuote = document.createElement('button');
      btnQuote.type = 'button';
      btnQuote.className = 'btn btn-secondary btn-xs';
      btnQuote.style.marginLeft = '4px';
      btnQuote.textContent = '询价';
      btnQuote.title = '实时拉取该饰品的 Steam 盘口底价与多时段均价';
      btnQuote.onclick = async () => {
        btnQuote.disabled = true;
        btnQuote.textContent = '…';
        try {
          const book = await parallax.fetchOrderBook({ market_hash_name: a.market_hash_name, currency: 'CNY' });
          const askCents = book.lowest_sell_order_cents ?? book.lowest_ask_cents;
          if (askCents > 0) {
            const prev = multiPeriodPriceCache.get(a.market_hash_name) || {};
            prev.now = askCents / 100;
            multiPeriodPriceCache.set(a.market_hash_name, prev);
          }
          if (currentSnapshot) await render(currentSnapshot);
        } catch (err) {
          el('inventory-feedback').textContent = `询价提示：${err.message}`;
        } finally {
          btnQuote.disabled = false;
          btnQuote.textContent = '询价';
        }
      };

      const btnHold = document.createElement('button');
      btnHold.type = 'button';
      btnHold.className = 'btn btn-primary btn-xs';
      btnHold.style.marginLeft = '4px';
      btnHold.textContent = '入账';
      btnHold.title = '将该库存饰品一键转入「持有饰品」台账并自动绑定真实 AssetID';
      btnHold.onclick = async () => {
        btnHold.disabled = true;
        try {
          const unitCents = costPrice && costPrice > 0 ? Math.round(costPrice * 100) : Math.max(50, Math.round((refMarketPrice || 5) * 75));
          const nowSec = Math.floor(Date.now() / 1000);
          const purchasedAt = Math.min(nowSec, Number(snapshot.observed_at || nowSec));
          const created = await parallax.createPurchaseLot(snapshot.account_id, {
            client_key: crypto.randomUUID(),
            item_key: a.market_hash_name,
            quantity: a.amount,
            cash_cost_cents: unitCents * a.amount,
            purchased_at: purchasedAt,
            purchase_ref: `inv_${snapshot.id.slice(0, 8)}_${a.asset_id}`
          });
          if (a.context_id === '2' && a.amount === 1 && created?.lot?.id) {
            await parallax.appendLotEvent(snapshot.account_id, created.lot.id, {
              client_key: crypto.randomUUID(),
              expected_revision: created.lot.revision,
              action: {kind: 'receive', inventory_snapshot_id: snapshot.id, asset_ids: [a.asset_id]}
            });
          }
          purchaseLotsCache.delete(snapshot.account_id);
          window.dispatchEvent(new CustomEvent('parallax:ledger-changed', {detail: {account: snapshot.account_id}}));
          location.hash = 'purchases';
        } catch (err) {
          el('inventory-feedback').textContent = `入账提示：${err.message}`;
          btnHold.disabled = false;
        }
      };

      tdLinks.append(steamA, document.createTextNode(' '), buffA, document.createTextNode(' '), btnQuote, btnHold);
    } else {
      tdLinks.textContent = '—';
    }
    tr.appendChild(tdLinks);

    fragment.appendChild(tr);
  }

  tbody.replaceChildren(fragment);
}

if (periodSelect) {
  periodSelect.onchange = () => {
    if (currentSnapshot) render(currentSnapshot);
  };
}

let autoLoaded = false;
async function history() {
  const id = account.value;
  el('inventory-history').replaceChildren();
  if (!id) return;
  const {items} = await parallax.inventories(id);
  if (account.value !== id) return;

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
        el('inventory-feedback').textContent = '已载入本地库存快照。';
      } catch (e) {
        if (account.value === id) el('inventory-feedback').textContent = e.message;
      }
    };
    el('inventory-history').append(b);
  }
  if (!items.length) el('inventory-history').append(node('p', '该账户尚无快照备份。点击「立即刷新」开始首次拉取。'));
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
    el('inventory-feedback').textContent = '最新库存快照已同步，已自动匹配饰品图标、冷却时间与多时段折扣率。';
    try {
      await history();
    } catch (e) {
      el('inventory-feedback').textContent = `快照已更新：${e.message}`;
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
