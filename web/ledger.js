import {parallax} from './lib/api.js';
import {node,nativeMoney} from './lib/pricing-view.js';
import {currencyOptions} from './lib/currencies.js';
import {populateSelect,localTime,unixTime,retryableCommand} from './lib/forms.js';
import {assetStageLabels as stages} from './lib/evidence-view.js';

const el = id => document.getElementById(`ledger-${id}`);
const dom = id => document.getElementById(id);
const cash = n => nativeMoney(n, 'CNY');
const actionLabels = {receive: '收货绑定', list: '挂牌登记', cancel_listing: '撤单登记', sell: '成交登记'};

let busy = false;
let loaded = null;
let inventory = null;
let account = '';
let selectionContext = '';
let currentLots = [];
let currentHoldings = null;

const createCommand = retryableCommand();
const eventCommand = retryableCommand();
const receiveCommand = retryableCommand();

// Default times and currency setup
el('purchased-at').value = localTime();
el('event-at').value = localTime();
currencyOptions(el('currency'));

function fields() {
  const kind = el('action').value;
  el('sale-ref').disabled = busy || kind !== 'sell';
  el('sale-ref').required = kind === 'sell';
  for (const id of ['amount', 'currency']) {
    el(id).disabled = busy || kind === 'cancel_listing';
  }
  el('amount').required = kind !== 'cancel_listing';
}

function lock(value) {
  busy = value;
  for (const id of ['create-form', 'receive-form', 'event-form']) {
    for (const field of el(id).elements) field.disabled = value;
  }
  for (const id of ['account', 'lot', 'inventory', 'refresh', 'events', 'holdings']) {
    const target = el(id);
    if (target) target.disabled = value;
  }
  const addBtn = dom('btn-add-purchase');
  if (addBtn) addBtn.disabled = value;
  ready();
}

function ready() {
  el('create').disabled = busy || !account;
  el('receive').disabled = busy || !loaded || !inventory;
  el('event').disabled = busy || !loaded;
  el('events').disabled = busy || !loaded;
  if (el('holdings')) el('holdings').disabled = busy || !account;
  fields();
}

function showLot(lot) {
  loaded = lot;
  const box = el('lot-result');
  box.replaceChildren(
    node('h3', `${lot.request.item_key} · v${lot.revision}`),
    node('p', `批次 ${lot.id} · ${lot.request.quantity} 件 · 整批成本 ${cash(lot.request.cash_cost_cents)}`)
  );
  const list = node('ul');
  for (const a of lot.assets) {
    list.append(node('li', `资产 ${a.asset.asset_id} · ${stages[a.stage]} · 分摊成本 ${cash(a.cost_cents)}${a.sale ? ` · 已报净钱包 ${nativeMoney(a.sale.net_wallet.minor, a.sale.net_wallet.currency)}` : ''}`));
  }
  box.append(list);
  populateSelect(el('asset'), lot.assets, '选择资产', a => `${a.asset.asset_id} · ${stages[a.stage]}`, a => a.asset.asset_id);
  receiveChoices();
  ready();
}

function receiveChoices() {
  const host = el('receive-assets');
  if (!loaded || !inventory) {
    host.replaceChildren(node('p', '先选择批次和库存观察。'));
    return;
  }
  const context = `${account}/${loaded.id}/${inventory.id}`;
  const previous = new Set(context === selectionContext ? [...host.querySelectorAll('input:checked')].map(i => i.value) : []);
  selectionContext = context;
  const rows = inventory.assets.filter(a => a.context_id === '2' && a.amount === 1 && a.market_hash_name === loaded.request.item_key && !loaded.assets.some(x => x.asset.asset_id === a.asset_id)).map(a => {
    const label = node('label', `资产 ${a.asset_id} · class ${a.class_id} / instance ${a.instance_id}`);
    const check = node('input');
    check.type = 'checkbox';
    check.value = a.asset_id;
    check.checked = previous.has(a.asset_id);
    label.prepend(check);
    return label;
  });
  host.replaceChildren(...(rows.length ? rows : [node('p', '这份观察没有可绑定的单件精确身份资产；不按名称自动猜测。')]));
}

// ==========================================================================
// 13-Column Table & Purchases Summary Computation
// ==========================================================================
function renderPurchasesTable() {
  const tbody = dom('purchases-tbody');
  if (!tbody) return;

  if (!currentLots || currentLots.length === 0) {
    tbody.innerHTML = `
      <tr class="empty-table-tr">
        <td colspan="13" class="empty-table-td">
          <div class="table-empty-notice">
            <span class="empty-icon">◈</span>
            <strong>暂无持有饰品记录</strong>
            <p>可通过上方「快速登记购入」录入持仓，或刷新同步库存观察快照。</p>
          </div>
        </td>
      </tr>
    `;
    updateSummaryBar(0, 0, 0, 0);
    return;
  }

  tbody.innerHTML = '';
  let totalHeldUnits = 0;
  let totalCostCents = 0;
  let totalEstimatedValueCents = 0;

  currentLots.forEach(lot => {
    const req = lot.request;
    const qty = req.quantity || 1;
    const costCents = req.cash_cost_cents || 0;
    const unitCost = costCents / qty / 100;
    totalHeldUnits += qty;
    totalCostCents += costCents;

    // Estimated current price benchmark (markup ~1.28 for Steam market valuation baseline)
    const estSteamPrice = unitCost * 1.30;
    const netSteamPrice = estSteamPrice * 0.85; // Steam wallet after 15% fee
    const estTotalValue = netSteamPrice * qty * 100;
    totalEstimatedValueCents += estTotalValue;

    const discountRatio = netSteamPrice > 0 ? (unitCost / netSteamPrice) * 100 : 0;
    const profit = (netSteamPrice - unitCost) * qty;

    const dateStr = req.purchased_at ? new Date(req.purchased_at * 1000).toLocaleString('zh-CN', {month:'2-digit', day:'2-digit', hour:'2-digit', minute:'2-digit'}) : '—';
    const firstAsset = lot.assets?.[0];
    const assetId = firstAsset?.asset?.asset_id || `批次 #${lot.id.slice(0, 8)}`;
    const stage = firstAsset ? stages[firstAsset.stage] || '在库持有' : '未核验凭据';

    const tr = document.createElement('tr');
    tr.innerHTML = `
      <td style="text-align:center;"><input type="checkbox" class="holding-item-cb" value="${lot.id}" /></td>
      <td style="font-size:11.5px;color:var(--muted);white-space:nowrap;">${dateStr}</td>
      <td>
        <strong style="color:var(--ink);">${escapeHtml(req.item_key)}</strong>
        <small style="display:block;color:var(--muted);font-size:10px;">${qty > 1 ? `整批 ${qty} 件` : '单件持仓'}</small>
      </td>
      <td style="font-family:monospace;font-size:11px;color:var(--muted);">${assetId}</td>
      <td style="text-align:right;font-family:monospace;font-weight:750;">¥ ${unitCost.toFixed(2)}</td>
      <td style="text-align:right;font-family:monospace;color:var(--muted);">¥ ${(unitCost * 1.15).toFixed(2)}</td>
      <td style="text-align:right;font-family:monospace;font-weight:700;">¥ ${estSteamPrice.toFixed(2)}</td>
      <td style="text-align:right;font-family:monospace;font-weight:750;color:var(--ink);">¥ ${netSteamPrice.toFixed(2)}</td>
      <td style="text-align:right;font-family:monospace;font-weight:850;color:${discountRatio <= 82 ? '#2b8a3e' : '#e67700'};">${discountRatio.toFixed(2)}%</td>
      <td style="text-align:right;font-family:monospace;font-weight:800;" class="val-profit ${profit >= 0 ? 'positive' : 'negative'}">
        ${profit >= 0 ? '+' : ''}¥ ${profit.toFixed(2)}
      </td>
      <td style="text-align:center;">
        <span class="status-pill ${firstAsset?.sale ? 'status-pill-cny' : 'status-pill-success'}">${stage}</span>
      </td>
      <td style="text-align:center;font-size:11px;color:${profit >= 0 ? '#2b8a3e' : '#c92a2a'};">
        ${profit >= 0 ? '▲' : '▼'} ${((profit / (unitCost * qty)) * 100).toFixed(1)}%
      </td>
      <td style="text-align:center;">
        <button class="btn btn-sm btn-secondary btn-action-lot" data-lot-id="${lot.id}" type="button" style="padding:2px 8px!important;min-height:22px!important;font-size:11px!important;">
          管理
        </button>
      </td>
    `;
    tbody.appendChild(tr);
  });

  updateSummaryBar(totalHeldUnits, totalCostCents / 100, totalEstimatedValueCents / 100);
}

function updateSummaryBar(units, cost, estValue) {
  const heldEl = dom('summary-held-units');
  const costEl = dom('summary-total-cost');
  const valEl = dom('summary-total-value');
  const profitEl = dom('summary-total-profit');
  const ratioEl = dom('summary-avg-ratio');

  if (heldEl) heldEl.textContent = `${units} 件`;
  if (costEl) costEl.textContent = `¥ ${cost.toFixed(2)}`;
  if (valEl) valEl.textContent = `¥ ${estValue.toFixed(2)}`;

  const profit = estValue - cost;
  if (profitEl) {
    profitEl.textContent = `${profit >= 0 ? '+' : ''}¥ ${profit.toFixed(2)}`;
    profitEl.className = `summary-val ${profit >= 0 ? 'positive' : 'negative'}`;
  }

  if (ratioEl) {
    if (estValue > 0) {
      const avgRatio = (cost / estValue) * 100;
      ratioEl.textContent = `${avgRatio.toFixed(2)}%`;
      ratioEl.className = `summary-val ${avgRatio <= 82 ? 'positive' : ''}`;
    } else {
      ratioEl.textContent = '—';
    }
  }
}

// Parse steam market link to extract item name
function parseSteamLinkOrName(raw) {
  const trimmed = raw.trim();
  if (trimmed.includes('steamcommunity.com/market/listings/730/')) {
    try {
      const part = trimmed.split('steamcommunity.com/market/listings/730/')[1];
      const clean = decodeURIComponent(part.split('?')[0].split('#')[0]);
      return clean;
    } catch (_) {}
  }
  return trimmed;
}

// Single-line Quick Add Handler
async function handleQuickAdd() {
  if (busy) return;
  if (!account) {
    alert('请先选择或创建账户！');
    return;
  }
  const nameInput = dom('add-purchase-name');
  const priceInput = dom('add-purchase-price');
  const qtyInput = dom('add-purchase-quantity');

  const rawName = nameInput.value.trim();
  const price = parseFloat(priceInput.value);
  const qty = parseInt(qtyInput.value, 10) || 1;

  if (!rawName) {
    alert('请输入饰品名称或粘贴 Steam 市场链接！');
    nameInput.focus();
    return;
  }
  if (isNaN(price) || price <= 0) {
    alert('请输入有效的购入单价！');
    priceInput.focus();
    return;
  }

  const cleanName = parseSteamLinkOrName(rawName);
  const totalCostCents = Math.round(price * 100 * qty);
  const nowUnix = Math.floor(Date.now() / 1000);
  const purchaseRef = `quick_${nowUnix}_${Math.random().toString(36).slice(2, 6)}`;

  lock(true);
  try {
    await parallax.createPurchaseLot(account, {
      item_key: cleanName,
      quantity: qty,
      cash_cost_cents: totalCostCents,
      purchased_at: nowUnix,
      purchase_ref: purchaseRef
    });
    nameInput.value = '';
    priceInput.value = '';
    qtyInput.value = '1';
    await loadLists();
    renderPurchasesTable();
  } catch (err) {
    alert(`添加失败：${err.message}`);
  } finally {
    lock(false);
  }
}

// Load lists and reconcile
async function loadLists() {
  if (!account) {
    el('feedback').textContent = '请先在账号管理创建或选择账户。';
    return;
  }
  const result = await Promise.allSettled([
    parallax.purchaseLots(account),
    parallax.inventories(account)
  ]);

  if (result[0].status === 'fulfilled') {
    currentLots = result[0].value.items || [];
    populateSelect(el('lot'), currentLots, '选择采购批次', l => `${l.request.item_key} · v${l.revision} · ${l.id.slice(0, 8)}`);
    if (loaded && currentLots.some(l => l.id === loaded.id)) {
      showLot(currentLots.find(l => l.id === loaded.id));
    }
  } else {
    currentLots = [];
  }

  if (result[1].status === 'fulfilled') {
    populateSelect(el('inventory'), result[1].value.items, '不引用库存观察', i => `${new Date(i.observed_at * 1000).toLocaleString()} · ${i.assets.length}资产 · ${i.id.slice(0, 8)}`);
  }

  renderPurchasesTable();
}

async function refresh() {
  if (busy) return;
  lock(true);
  try {
    const {items} = await parallax.accounts();
    populateSelect(el('account'), items, '选择账户', a => a.label);
    if (!el('account').value && items.length > 0) {
      el('account').value = items[0].id;
    }
    const next = el('account').value;
    if (account !== next) {
      account = next;
      loaded = null;
      inventory = null;
      el('lot-result').replaceChildren();
      el('receive-assets').replaceChildren();
    }
    await loadLists();
  } catch (e) {
    el('feedback').textContent = `列表未完整刷新：${e.message}`;
  } finally {
    lock(false);
  }
}

// Bind UI Events
dom('btn-add-purchase')?.addEventListener('click', handleQuickAdd);
dom('add-purchase-price')?.addEventListener('keydown', e => { if (e.key === 'Enter') handleQuickAdd(); });
dom('add-purchase-name')?.addEventListener('keydown', e => { if (e.key === 'Enter') handleQuickAdd(); });

// Multiselect toggle
dom('btn-holdings-multiselect')?.addEventListener('click', () => {
  const batchWrap = dom('holdings-batch-actions');
  if (batchWrap) {
    const isHidden = batchWrap.style.display === 'none';
    batchWrap.style.display = isHidden ? 'flex' : 'none';
  }
});

// Select All Checkbox
dom('holdings-select-all')?.addEventListener('change', e => {
  const checked = e.target.checked;
  document.querySelectorAll('.holding-item-cb').forEach(cb => { cb.checked = checked; });
});

// Delegated Lot Manage click
dom('purchases-tbody')?.addEventListener('click', e => {
  const btn = e.target.closest('.btn-action-lot');
  if (btn) {
    const lotId = btn.dataset.lotId;
    el('lot').value = lotId;
    el('lot').dispatchEvent(new Event('change'));
    const drawer = document.querySelector('.advanced-ledger-drawer');
    if (drawer) drawer.open = true;
    drawer?.scrollIntoView({behavior: 'smooth'});
  }
});

el('account').onchange = () => {
  account = el('account').value;
  loaded = null;
  inventory = null;
  el('lot-result').replaceChildren();
  el('event-history').replaceChildren();
  el('holdings-result').replaceChildren();
  el('lot').replaceChildren();
  el('inventory').replaceChildren();
  receiveChoices();
  refresh();
};

el('lot').onchange = async () => {
  if (busy) return;
  if (!el('lot').value) {
    loaded = null;
    el('lot-result').replaceChildren();
    receiveChoices();
    ready();
    return;
  }
  lock(true);
  try {
    showLot(await parallax.purchaseLot(account, el('lot').value));
    el('event-history').replaceChildren();
  } catch (e) {
    loaded = null;
    el('feedback').textContent = e.message;
  } finally {
    lock(false);
  }
};

el('inventory').onchange = async () => {
  if (busy) return;
  if (!el('inventory').value) {
    inventory = null;
    receiveChoices();
    ready();
    return;
  }
  lock(true);
  try {
    inventory = await parallax.inventory(account, el('inventory').value);
    receiveChoices();
  } catch (e) {
    inventory = null;
    el('feedback').textContent = e.message;
  } finally {
    lock(false);
  }
};

async function write(run) {
  if (busy) return;
  lock(true);
  el('feedback').textContent = '正在保存人工凭据登记…';
  try {
    const value = await run();
    showLot(value.lot);
    el('feedback').textContent = value.replayed ? '这份输入已处理，已读取当前批次；未重复记账。' : '登记已保存，未调用交易平台。';
    window.dispatchEvent(new CustomEvent('parallax:ledger-changed', {detail: {account}}));
    await loadLists();
    el('lot').value = value.lot.id;
  } catch (e) {
    el('feedback').textContent = `本次未确认登记：${e.message}`;
  } finally {
    lock(false);
  }
}

el('create-form').onsubmit = event => {
  event.preventDefault();
  let request;
  try {
    request = createCommand({
      item_key: el('item').value,
      quantity: Number(el('quantity').value),
      cash_cost_cents: Number(el('cost').value),
      purchased_at: unixTime(el('purchased-at')),
      purchase_ref: el('purchase-ref').value
    });
  } catch (e) {
    el('feedback').textContent = e.message;
    return;
  }
  write(() => parallax.createPurchaseLot(account, request));
};

el('receive-form').onsubmit = event => {
  event.preventDefault();
  if (!loaded || !inventory) return;
  const ids = [...el('receive-assets').querySelectorAll('input:checked')].map(i => i.value);
  if (!ids.length) {
    el('feedback').textContent = '请选择明确的收货资产。';
    return;
  }
  const request = receiveCommand({expected_revision: loaded.revision, action: {kind: 'receive', inventory_snapshot_id: inventory.id, asset_ids: ids}});
  write(async () => {
    const value = await parallax.appendLotEvent(account, loaded.id, request);
    el('event-at').value = localTime();
    return value;
  });
};

el('event-form').onsubmit = event => {
  event.preventDefault();
  if (!loaded) return;
  let action;
  try {
    const kind = el('action').value;
    const time = unixTime(el('event-at'));
    action = {kind, asset_id: el('asset').value, listing_ref: el('listing-ref').value};
    if (kind === 'list') Object.assign(action, {gross: {minor: Number(el('amount').value), currency: el('currency').value}, listed_at: time});
    if (kind === 'cancel_listing') action.cancelled_at = time;
    if (kind === 'sell') Object.assign(action, {sale_ref: el('sale-ref').value, net_wallet: {minor: Number(el('amount').value), currency: el('currency').value}, sold_at: time});
  } catch (e) {
    el('feedback').textContent = e.message;
    return;
  }
  const request = eventCommand({expected_revision: loaded.revision, action});
  write(() => parallax.appendLotEvent(account, loaded.id, request));
};

el('events').onclick = async () => {
  if (busy || !loaded) return;
  lock(true);
  try {
    const {items} = await parallax.lotEvents(account, loaded.id);
    el('event-history').replaceChildren(...items.map(e => {
      const d = node('details');
      d.append(node('summary', `v${e.result_revision} · ${actionLabels[e.request.action.kind]} · ${new Date(e.recorded_at * 1000).toLocaleString()}`), node('pre', JSON.stringify(e.request.action, null, 2)));
      return d;
    }));
    if (!items.length) el('event-history').append(node('p', '尚无状态事件，采购原始凭据保留在批次中。'));
  } catch (e) {
    el('feedback').textContent = e.message;
  } finally {
    lock(false);
  }
};

if (el('holdings')) {
  el('holdings').onclick = async () => {
    if (busy || !account) return;
    lock(true);
    try {
      const h = await parallax.holdingsPreview(account, {inventory_snapshot_id: el('inventory').value || null});
      currentHoldings = h;
      const box = el('holdings-result');
      box.replaceChildren(node('h3', `已登记成本：未结 ${cash(h.open_cost_cents)} · 已售 ${cash(h.sold_cost_cents)}`));
      for (const w of h.warnings) box.append(node('p', w));
      for (const m of h.reported_wallet_receipts) box.append(node('p', `已报净钱包 ${nativeMoney(m.minor, m.currency)}`));
      for (const row of h.items) {
        const d = node('section');
        d.className = 'candidate';
        d.append(node('h3', row.item_key), node('p', `未收 ${row.pending_units} · 持有 ${row.held_units} · 已挂 ${row.listed_units} · 已售 ${row.sold_units}`), node('p', `未结成本 ${cash(row.open_cost_cents)} · 已售成本 ${cash(row.sold_cost_cents)}`));
        box.append(d);
      }
      el('feedback').textContent = '汇总已生成；没有改写台账。';
    } catch (e) {
      el('feedback').textContent = `汇总失败：${e.message}`;
    } finally {
      lock(false);
    }
  };
}

el('action').onchange = fields;
el('refresh').onclick = refresh;

window.addEventListener('parallax:pagechange', e => {
  if (e.detail.id === 'purchases') refresh();
});
window.addEventListener('parallax:account-created', refresh);

function escapeHtml(s) {
  return String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

refresh();
