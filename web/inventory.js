import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const account=el('inventory-account');
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
let fetching=false;
function render(snapshot) {
  const complete=snapshot.coverage.length===2&&snapshot.coverage.every(c=>c.exhausted);
  const box=el('inventory-result');
  box.replaceChildren(node('h3',complete?'双区分页已读完 · 非原子库存':'读取已截断 · 仅作局部观察'),
    node('p',`账户 ${snapshot.account_id} · Steam ${snapshot.steam_id} · 快照 ${snapshot.id}`),
    node('p',`观察于 ${new Date(snapshot.observed_at*1000).toLocaleString()} · ${snapshot.assets.length} 个资产条目 · ${snapshot.assets.reduce((n,a)=>n+a.amount,0)} 件`));
  for(const c of snapshot.coverage)box.append(node('p',`库存区 ${c.context_id} · ${c.pages_read} 页 · 已见 ${c.observed_assets} / 平台报告 ${c.reported_total} 个资产 · ${c.exhausted?'本次分页读完':'到达页数预算'}`));
  for(const warning of snapshot.warnings)box.append(node('p',warning));
  const list=node('ul');
  for(const a of snapshot.assets.slice(0,100)) {
    const transferable=a.context_id==='2'&&a.reported_tradable&&!a.restriction_note_present;
    const row=node('li',`${a.market_hash_name??'无市场名称，保留原始资产身份'} · ${a.amount} 件 · ${a.context_id}/${a.asset_id} · ${transferable?'报告可交易':'受保护、受限或状态未知'} · 市场标志 ${a.reported_marketable?'是':'否'} · 解锁时间未知`);
    if(a.market_hash_name&&a.reported_marketable){
      const research=node('button','研究价格历史');research.type='button';research.className='secondary';
      research.onclick=()=>window.dispatchEvent(new CustomEvent('parallax:inventory-history',{detail:{account_id:snapshot.account_id,inventory_snapshot_id:snapshot.id,item_key:a.market_hash_name}}));
      row.append(research);
    }
    list.append(row);
  }
  box.append(list);
  if(snapshot.assets.length>100)box.append(node('p','页面仅展示前 100 个资产，完整内容保留在快照 API。'));
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
