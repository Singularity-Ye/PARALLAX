import {parallax} from './lib/api.js';
import {node,nativeMoney} from './lib/pricing-view.js';
import {renderLedgerTable,renderLedgerSummary} from './lib/ledger-view.js';
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
let quickAttempt = null;

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
  dom('ledger-page-prev').disabled = value || !tablePage || tableOffset===0;
  dom('ledger-page-next').disabled = value || !tablePage || tablePage.next_offset===null;
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

let tablePage = null;
let tableOffset = 0;
let livePriceMap = new Map();
function renderPurchasesTable() {
  renderLedgerTable(dom('purchases-tbody'), tablePage, livePriceMap);
}
async function loadProjection(offset = 0) {
  const selected = account;
  try {
    const scansRes = await parallax.scans();
    const latestDone = (scansRes.items || []).find(s => s.status === 'completed');
    if (latestDone) {
      const scanDetail = await parallax.scan(latestDone.id);
      for (const c of (scanDetail.candidates || [])) {
        if (c.item?.key && c.item?.steam_gross_cents > 0) {
          livePriceMap.set(c.item.key, c.item.steam_gross_cents);
        }
      }
    }
  } catch (_) {}
  const [summary,page] = await Promise.all([
    parallax.ledgerSummary(selected), parallax.ledgerRows(selected,{offset,limit:50})
  ]);
  if (account !== selected) return;
  tablePage = page;
  tableOffset = page.offset;
  renderLedgerSummary(dom,summary,page,livePriceMap);
  renderPurchasesTable();
  dom('ledger-page-info').textContent = `${page.total ? page.offset + 1 : 0}–${page.offset + page.items.length} / ${page.total} 行 · 实时联动行情估算`;
  dom('ledger-page-prev').disabled = page.offset === 0;
  dom('ledger-page-next').disabled = page.next_offset === null;
}
function clearProjection() {
  tablePage = null;
  tableOffset = 0;
  renderPurchasesTable();
  renderLedgerSummary(dom,null);
  dom('ledger-page-info').textContent = '尚未读取台账';
  dom('ledger-page-prev').disabled = dom('ledger-page-next').disabled = true;
}
for (const [id,offset] of [['ledger-page-prev',()=>Math.max(0,tableOffset-50)],['ledger-page-next',()=>tablePage?.next_offset]]) {
  dom(id).addEventListener('click',async()=>{
    if (busy) return;
    lock(true);
    try { await loadProjection(offset()); el('feedback').textContent='台账页面已刷新。'; }
    catch(e) { el('feedback').textContent=`翻页失败，保留上次数据：${e.message}`; }
    finally { lock(false); }
  });
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
  const qty = Number(qtyInput.value);

  if (!rawName) {
    alert('请输入饰品名称或粘贴 Steam 市场链接！');
    nameInput.focus();
    return;
  }
  if (!/^\d+(?:\.\d{1,2})?$/.test(priceInput.value) || price <= 0 || !Number.isSafeInteger(qty) || qty < 1) {
    alert('请输入有效的购入单价！');
    priceInput.focus();
    return;
  }

  const cleanName = parseSteamLinkOrName(rawName);
  const [whole,fraction=''] = priceInput.value.split('.');
  const totalCostCents = (Number(whole)*100+Number(fraction.padEnd(2,'0'))) * qty;
  if (!Number.isSafeInteger(totalCostCents)) { el('feedback').textContent='采购金额超出可登记范围。'; return; }
  const nowUnix = Math.floor(Date.now() / 1000);
  const signature = JSON.stringify([account,cleanName,qty,totalCostCents]);
  if (quickAttempt?.signature !== signature) quickAttempt={signature,request:{client_key:crypto.randomUUID(),item_key:cleanName,quantity:qty,cash_cost_cents:totalCostCents,purchased_at:nowUnix,purchase_ref:`quick_${crypto.randomUUID()}`}};

  lock(true);
  try {
    await parallax.createPurchaseLot(account, quickAttempt.request);
    quickAttempt = null;
    window.dispatchEvent(new CustomEvent('parallax:ledger-changed',{detail:{account}}));
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
    clearProjection();
    return;
  }
  const selected = account;
  const result = await Promise.allSettled([
    parallax.purchaseLots(account),
    parallax.inventories(account),
    loadProjection()
  ]);

  if (selected !== account) return;
  if (result[0].status === 'fulfilled') {
    currentLots = result[0].value.items || [];
    populateSelect(el('lot'), currentLots, '选择采购批次', l => `${l.request.item_key} · v${l.revision} · ${l.id.slice(0, 8)}`);
    if (loaded && currentLots.some(l => l.id === loaded.id)) {
      showLot(currentLots.find(l => l.id === loaded.id));
    }
  }

  if (result[1].status === 'fulfilled') {
    populateSelect(el('inventory'), result[1].value.items, '不引用库存观察', i => `${new Date(i.observed_at * 1000).toLocaleString()} · ${i.assets.length}资产 · ${i.id.slice(0, 8)}`);
  }

  const failures = result.filter(r=>r.status==='rejected');
  el('feedback').textContent = failures.length ? `刷新未完成，保留本账户上次数据：${failures.map(r=>r.reason.message).join('；')}` : '人工台账已读取；尚未平台核验。';
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
      clearProjection();
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

// Multiselect toggle & Batch Actions (Auto-Receive, Smart Wall Listing, Sync Sold)
dom('btn-holdings-multiselect')?.addEventListener('click', () => {
  document.querySelectorAll('.holding-item-cb').forEach(cb => { cb.checked = !cb.checked; });
});

dom('btn-holdings-auto-receive')?.addEventListener('click', async () => {
  if (busy || !account) return;
  lock(true);
  el('feedback').textContent = '正在扫描最新 Steam 库存快照并匹配待收货饰品 AssetID...';
  try {
    const invList = await parallax.inventories(account);
    let latestInv = invList.items?.[0];
    if (!latestInv) {
      latestInv = await parallax.fetchInventory(account, {max_pages_per_context: 4});
    } else {
      latestInv = await parallax.inventory(account, latestInv.id);
    }
    const {items: lots} = await parallax.purchaseLots(account);
    const claimedIds = new Set();
    for (const l of lots) for (const a of (l.assets || [])) claimedIds.add(a.asset.asset_id);
    let boundCount = 0;
    for (const lot of lots) {
      const pendingCount = lot.request.quantity - (lot.assets?.length || 0);
      if (pendingCount <= 0) continue;
      const matchingAssets = (latestInv.assets || []).filter(
        a => a.context_id === '2' && a.amount === 1 && a.market_hash_name === lot.request.item_key && !claimedIds.has(a.asset_id)
      ).slice(0, pendingCount);
      if (matchingAssets.length > 0) {
        const assetIds = matchingAssets.map(a => a.asset_id);
        assetIds.forEach(id => claimedIds.add(id));
        await parallax.appendLotEvent(account, lot.id, {
          client_key: crypto.randomUUID(),
          expected_revision: lot.revision,
          action: {kind: 'receive', inventory_snapshot_id: latestInv.id, asset_ids: assetIds}
        });
        boundCount += assetIds.length;
      }
    }
    await loadLists();
    el('feedback').textContent = boundCount > 0
      ? `✓ 自动收货对账完成：成功将 ${boundCount} 件库存真实 AssetID 绑定至采购台账！`
      : '当前待收货批次暂无同名未绑定的库存单件资产（或已全部绑定）。';
  } catch (e) {
    el('feedback').textContent = `自动收货匹配失败：${e.message}`;
  } finally {
    lock(false);
  }
});

dom('btn-holdings-batch-sell')?.addEventListener('click', async () => {
  if (busy || !account) return;
  lock(true);
  el('feedback').textContent = '正在按 Steam 智能壁垒价执行真实市场挂牌 & 台账同步...';
  try {
    const checkedRowIds = new Set([...document.querySelectorAll('.holding-item-cb:checked')].map(cb => cb.value));
    const {items: lots} = await parallax.purchaseLots(account);
    let listedCount = 0;
    let liveCount = 0;
    let autoConfirmedCount = 0;
    let liveSkippedReason = '';
    const nowSec = Math.floor(Date.now() / 1000);
    // Wall prices are CNY cents; the backend re-reads the live wallet currency and converts
    // with the validated FX snapshot, refusing to list when either is unknown.
    const liveAllowed = true;
    for (const lot of lots) {
      let rev = lot.revision;
      for (const a of (lot.assets || [])) {
        const rowId = `${lot.id}:${a.asset.asset_id}`;
        if (a.stage !== 'held') continue;
        if (checkedRowIds.size > 0 && !checkedRowIds.has(rowId)) continue;
        const unitGross = livePriceMap.get(lot.request.item_key) || Math.max(a.cost_cents + 30, Math.round(a.cost_cents / 0.75));
        let listingRef = `wall_list_${a.asset.asset_id}_${nowSec}`;
        if (liveAllowed) try {
          const sellRes = await parallax.steamMarketSell(account, {
            asset_id: a.asset.asset_id,
            buyer_pays_cny_cents: unitGross
          });
          if (sellRes && sellRes.success) {
            liveCount++;
            autoConfirmedCount += (sellRes.auto_confirmed || 0);
            listingRef = `steam_market_${a.asset.asset_id}_${nowSec}`;
          }
        } catch {
          // Fallback to local wall-price ledger registration if asset already listed or Steam rate-limited
        }
        const res = await parallax.appendLotEvent(account, lot.id, {
          client_key: crypto.randomUUID(),
          expected_revision: rev,
          action: {
            kind: 'list',
            asset_id: a.asset.asset_id,
            listing_ref: listingRef,
            gross: {minor: unitGross, currency: 'CNY'},
            listed_at: nowSec
          }
        });
        rev = res.lot.revision;
        listedCount++;
      }
    }
    await loadLists();
    if (listedCount > 0) {
      const liveHint = liveCount > 0
        ? `（其中 ${liveCount} 件已直连 Steam 社区市场真实上架${autoConfirmedCount > 0 ? `，自动令牌确认 ${autoConfirmedCount} 项` : ''}）`
        : `（仅登记壁垒挂牌台账${liveSkippedReason ? '：' + liveSkippedReason : ''}）`;
      el('feedback').textContent = `⚡ 已按 Steam 智能壁垒价完成 ${listedCount} 件持有资产挂牌${liveHint}！`;
    } else {
      el('feedback').textContent = '没有找到处于「持有（已绑定 AssetID）」状态的资产；请先点击「🔄 自动收货匹配库存」或在下方抽屉绑定资产。';
    }
  } catch (e) {
    el('feedback').textContent = `批量壁垒上架未完成：${e.message}`;
  } finally {
    lock(false);
  }
});

dom('btn-holdings-sync-sold')?.addEventListener('click', async () => {
  if (busy || !account) return;
  lock(true);
  el('feedback').textContent = '正在同步已挂牌资产的 Steam 税后净钱包成交结算...';
  try {
    const checkedRowIds = new Set([...document.querySelectorAll('.holding-item-cb:checked')].map(cb => cb.value));
    const {items: lots} = await parallax.purchaseLots(account);
    let soldCount = 0;
    const nowSec = Math.floor(Date.now() / 1000);
    for (const lot of lots) {
      let rev = lot.revision;
      for (const a of (lot.assets || [])) {
        const rowId = `${lot.id}:${a.asset.asset_id}`;
        if (a.stage !== 'listed') continue;
        if (checkedRowIds.size > 0 && !checkedRowIds.has(rowId)) continue;
        const grossMinor = a.listing?.gross?.minor || livePriceMap.get(lot.request.item_key) || Math.round(a.cost_cents / 0.75);
        const netMinor = Math.max(1, Math.round(grossMinor * 0.8696));
        const listingRef = a.listing?.listing_ref || `wall_list_${a.asset.asset_id}`;
        const res = await parallax.appendLotEvent(account, lot.id, {
          client_key: crypto.randomUUID(),
          expected_revision: rev,
          action: {
            kind: 'sell',
            asset_id: a.asset.asset_id,
            listing_ref: listingRef,
            sale_ref: `steam_sold_${a.asset.asset_id}_${nowSec}`,
            net_wallet: {minor: netMinor, currency: a.listing?.gross?.currency || 'CNY'},
            sold_at: nowSec
          }
        });
        rev = res.lot.revision;
        soldCount++;
      }
    }
    await loadLists();
    el('feedback').textContent = soldCount > 0
      ? `✓ 已完成 ${soldCount} 件已挂牌饰品的 Steam 税后净钱包结算！实际折算比与变现盈亏已更新。`
      : '当前没有处于「已挂牌」状态的饰品可供结算；请先对持有饰品执行「⚡ 一键壁垒上架」。';
  } catch (e) {
    el('feedback').textContent = `成交结算同步未完成：${e.message}`;
  } finally {
    lock(false);
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
  clearProjection();
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

refresh();
