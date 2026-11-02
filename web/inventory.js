import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const account=el('inventory-account');
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
let fetching=false;
function render(snapshot) {
  const complete = snapshot.coverage.length === 2 && snapshot.coverage.every(c => c.exhausted);
  const box = el('inventory-result');
  if (box) {
    box.replaceChildren(
      node('p', `快照 ${snapshot.id.slice(0, 8)} · 观察于 ${new Date(snapshot.observed_at * 1000).toLocaleString()} · 账户 ${snapshot.account_id} · SteamID ${snapshot.steam_id}`, 'hint')
    );
    for (const c of snapshot.coverage) {
      box.append(node('p', `库存区 ${c.context_id} · ${c.pages_read} 页 · 已见 ${c.observed_assets} / 平台报告 ${c.reported_total} 个资产 · ${c.exhausted ? '全量已读完' : '到达页数预算'}`, 'hint'));
    }
    for (const warning of snapshot.warnings) {
      box.append(node('p', warning, 'hint error'));
    }
  }

  // Update Summary Capsules
  const totalUnits = snapshot.assets.reduce((n, a) => n + a.amount, 0);
  if (el('inv-total-count')) el('inv-total-count').textContent = totalUnits;
  if (el('inv-asset-count')) el('inv-asset-count').textContent = snapshot.assets.length;
  if (el('inv-status-label')) el('inv-status-label').textContent = complete ? '全量同步完成' : '局部截断观察';

  // Render Table Rows
  const tbody = el('inventory-tbody');
  if (!tbody) return;

  if (!snapshot.assets.length) {
    tbody.innerHTML = '<tr class="empty-row"><td colspan="8" class="empty-cell">当前快照下没有观察到任何饰品资产。</td></tr>';
    return;
  }

  const fragment = document.createDocumentFragment();
  for (const a of snapshot.assets.slice(0, 200)) {
    const tr = document.createElement('tr');
    tr.className = 'inv-row';

    // 1. Thumbnail
    const tdThumb = document.createElement('td');
    tdThumb.className = 'col-thumb';
    const img = document.createElement('img');
    img.className = 'inv-item-img';
    img.alt = a.market_hash_name || 'Item';
    img.loading = 'lazy';
    if (a.market_hash_name) {
      img.src = `https://community.cloudflare.steamstatic.com/economy/image/${encodeURIComponent(a.market_hash_name)}/64fx64f`;
      img.onerror = () => {
        img.src = 'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="%2394a3b8" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline></svg>';
      };
    } else {
      img.src = 'data:image/svg+xml;utf8,<svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="%2394a3b8" stroke-width="1.5"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline></svg>';
    }
    tdThumb.appendChild(img);
    tr.appendChild(tdThumb);

    // 2. Item Name
    const tdName = document.createElement('td');
    tdName.className = 'col-inv-name';
    const strong = document.createElement('strong');
    strong.textContent = a.market_hash_name || '未命名资产条目';
    tdName.appendChild(strong);
    tr.appendChild(tdName);

    // 3. Context & Asset ID
    const tdId = document.createElement('td');
    tdId.className = 'col-mono';
    tdId.innerHTML = `<span class="context-tag">区 ${a.context_id}</span> <small class="text-muted">${a.asset_id.slice(-6)}</small>`;
    tdId.title = `AssetID: ${a.asset_id} | ClassID: ${a.class_id}`;
    tr.appendChild(tdId);

    // 4. Amount
    const tdAmt = document.createElement('td');
    tdAmt.className = 'col-mono text-center';
    tdAmt.textContent = a.amount;
    tr.appendChild(tdAmt);

    // 5. Tradable status
    const tdTrade = document.createElement('td');
    const tradePill = document.createElement('span');
    const transferable = a.context_id === '2' && a.reported_tradable && !a.restriction_note_present;
    tradePill.className = transferable ? 'status-pill status-pill-success' : 'status-pill status-pill-active';
    tradePill.textContent = transferable ? '可交易' : '受限/冷却';
    tdTrade.appendChild(tradePill);
    tr.appendChild(tdTrade);

    // 6. Marketable status
    const tdMarket = document.createElement('td');
    const marketPill = document.createElement('span');
    marketPill.className = a.reported_marketable ? 'status-pill status-pill-success' : 'status-pill status-pill-active';
    marketPill.textContent = a.reported_marketable ? '可挂牌' : '不可出售';
    tdMarket.appendChild(marketPill);
    tr.appendChild(tdMarket);

    // 7. Market Links (Steam / Buff)
    const tdLinks = document.createElement('td');
    tdLinks.className = 'col-links';
    if (a.market_hash_name) {
      const steamA = document.createElement('a');
      steamA.href = `https://steamcommunity.com/market/listings/730/${encodeURIComponent(a.market_hash_name)}`;
      steamA.target = '_blank';
      steamA.rel = 'noopener';
      steamA.className = 'ext-link';
      steamA.textContent = 'Steam ↗';

      const buffA = document.createElement('a');
      buffA.href = `https://buff.163.com/market/csgo?tab=selling&search=${encodeURIComponent(a.market_hash_name)}`;
      buffA.target = '_blank';
      buffA.rel = 'noopener';
      buffA.className = 'ext-link';
      buffA.textContent = 'BUFF ↗';

      tdLinks.append(steamA, document.createTextNode(' '), buffA);
    } else {
      tdLinks.textContent = '—';
    }
    tr.appendChild(tdLinks);

    // 8. Actions
    const tdAction = document.createElement('td');
    tdAction.className = 'col-actions';
    if (a.market_hash_name && a.reported_marketable) {
      const btnHist = document.createElement('button');
      btnHist.type = 'button';
      btnHist.className = 'btn btn-secondary btn-xs';
      btnHist.textContent = '行情历史';
      btnHist.onclick = () => window.dispatchEvent(new CustomEvent('parallax:inventory-history', {
        detail: { account_id: snapshot.account_id, inventory_snapshot_id: snapshot.id, item_key: a.market_hash_name }
      }));

      const btnWatch = document.createElement('button');
      btnWatch.type = 'button';
      btnWatch.className = 'btn btn-secondary btn-xs';
      btnWatch.textContent = '加关注';
      btnWatch.onclick = () => window.dispatchEvent(new CustomEvent('parallax:watch-item', {
        detail: { account_id: snapshot.account_id, item_key: a.market_hash_name }
      }));

      tdAction.append(btnHist, document.createTextNode(' '), btnWatch);
    } else {
      tdAction.textContent = '—';
    }
    tr.appendChild(tdAction);

    fragment.appendChild(tr);
  }

  tbody.replaceChildren(fragment);
  if (snapshot.assets.length > 200 && box) {
    box.append(node('p', '页面展示前 200 个资产条目，完整数据保留在快照记录。', 'hint'));
  }
}
async function history() {
  const id=account.value;el('inventory-history').replaceChildren();if(!id)return;
  const {items}=await parallax.inventories(id);if(account.value!==id)return;
  for(const snapshot of items) {
    const b=node('button',`${new Date(snapshot.observed_at*1000).toLocaleString()} · ${snapshot.assets.length} 个资产 · ${snapshot.coverage.every(c=>c.exhausted)?'双区已读完':'局部'}`);b.type='button';b.className='history-row';
    b.onclick=async()=>{try{const saved=await parallax.inventory(id,snapshot.id);if(account.value!==id)return;render(saved);el('inventory-feedback').textContent='历史快照；未重新请求平台或确认当前账户状态。';}catch(e){if(account.value===id)el('inventory-feedback').textContent=e.message;}};
    el('inventory-history').append(b);
  }
  if(!items.length)el('inventory-history').append(node('p','该账户尚无库存快照。'));
}
async function loadAccounts() {
  const preferred=account.value;const {items}=await parallax.accounts();account.replaceChildren();
  for(const a of items){const o=node('option',a.label);o.value=a.id;account.append(o);}
  if(items.some(a=>a.id===preferred))account.value=preferred;
  el('inventory-fetch').disabled=fetching||!account.value;
  await history();
}
account.onchange=()=>{el('inventory-result').replaceChildren();el('inventory-feedback').textContent='';history().catch(e=>el('inventory-feedback').textContent=e.message);};
el('inventory-form').onsubmit=async event=>{
  event.preventDefault();if(!account.value)return;
  const id=account.value;fetching=true;account.disabled=true;el('inventory-fetch').disabled=true;
  el('inventory-feedback').textContent='正在读取两个库存区，分页间隔至少一秒；失败后保留已有结果。';
  try {
    const snapshot=await parallax.fetchInventory(id,{max_pages_per_context:Number(el('inventory-pages').value)});
    if(account.value===id)render(snapshot);
    el('inventory-feedback').textContent='库存观察快照已保存。未替换预算仓位，未发货、上架或创建订单。';
    try{await history();}catch(e){el('inventory-feedback').textContent=`快照已保存，历史刷新失败：${e.message}`;}
  }catch(e){el('inventory-feedback').textContent=`本次未保存新快照：${e.message}；输入及原结果保留。`;}
  finally{fetching=false;account.disabled=false;el('inventory-fetch').disabled=!account.value;}
};
window.addEventListener('parallax:account-created',()=>loadAccounts().catch(e=>el('inventory-feedback').textContent=e.message));
loadAccounts().catch(e=>el('inventory-feedback').textContent=e.message);
