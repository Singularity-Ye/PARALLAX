import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const account=el('price-history-account');
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
let fetching=false;
const day=d=>new Date(d*86400000).toISOString().slice(0,10);
const money=m=>m?`${m.currency} ${(m.minor/100).toFixed(2)}`:'未知';
export function renderHistoryCandidates(report) {
  const previous=el('price-history-item').value;el('price-history-scan').value=report.id;
  el('price-history-item').replaceChildren();
  if(report.status==='completed'&&report.request.source==='steamdt') {
    for(const {item} of report.candidates){const o=node('option',item.key);o.value=item.key;el('price-history-item').append(o);}
  }
  if([...el('price-history-item').options].some(o=>o.value===previous))el('price-history-item').value=previous;
  el('price-history-fetch').disabled=fetching||!account.value||!el('price-history-item').value;
  if(report.request.source==='demo')el('price-history-feedback').textContent='演示扫描禁止向 Steam 请求虚构商品的历史。';
}
function render(snapshot) {
  const a=snapshot.analysis,s=snapshot.series,r=a.risk;
  const status={unknown_currency:'币种含糊或单位未适配',no_usable_daily_prices:'没有可用日价格',insufficient_pairs:'缺少七自然日配对',historical_pairs_available:'已计算历史配对'};
  const box=el('price-history-result');box.replaceChildren(node('h3',snapshot.request.item_key),
    node('p',`账户 ${snapshot.account_id} · 快照 ${snapshot.id} · 读取于 ${new Date(snapshot.observed_at*1000).toLocaleString()}`),
    node('p',`${status[a.status]??a.status} · 币种 ${s.currency??'未知'} · UTC 窗口 ${day(s.from_utc_day)} 至 ${day(s.until_utc_day_exclusive)}（不含结束日）`),
    node('p',`响应 ${s.response_rows} 条 · 窗口内 ${s.points.length} 条 · 排除窗口前 ${s.excluded_before_window} 条、当天 ${s.excluded_open_day} 条 · 缺少 ${a.missing_calendar_days} 个日期`),
    node('p',`七日配对 ${r?.sample_count??0} 个 · P10 ${r?.return_p10_bps==null?'未知':(r.return_p10_bps/100).toFixed(2)+'%'} · 参考历史日 ${a.reference_utc_day==null?'未知':day(a.reference_utc_day)} · 压力估算价 ${money(r?.conservative_gross)}`));
  for(const warning of a.warnings)box.append(node('p',warning));
  if(a.below_minor_unit_days)box.append(node('p',`${a.below_minor_unit_days} 个日期价格低于已适配最小单位，未当作零价。`));
  const list=node('ul');for(const b of a.daily_bars.slice(-14))list.append(node('li',`${day(b.utc_day)} · 日量加权样本价 ${money(b.vwap)} · 已知量 ${b.volume} · 零量样本 ${b.zero_volume_samples} · 缺量样本 ${b.missing_volume_samples}`));box.append(list);
  if(a.daily_bars.length>14)box.append(node('p','页面只展示最近 14 个有记录日期，完整数据保存在快照 API。'));
}
async function history() {
  const id=account.value;el('price-history-saved').replaceChildren();if(!id)return;
  const {items}=await parallax.priceHistories(id);if(account.value!==id)return;
  for(const snapshot of items) {
    const b=node('button',`${snapshot.request.item_key} · ${new Date(snapshot.observed_at*1000).toLocaleString()}`);b.type='button';b.className='history-row';
    b.onclick=async()=>{try{const saved=await parallax.priceHistory(id,snapshot.id);if(account.value!==id)return;render(saved);el('price-history-feedback').textContent='保存时的历史研究结果，未重新请求或计算当前行情。';}catch(e){if(account.value===id)el('price-history-feedback').textContent=e.message;}};
    el('price-history-saved').append(b);
  }
  if(!items.length)el('price-history-saved').append(node('p','该账户尚无价格历史研究快照。'));
}
async function loadAccounts() {
  const previous=account.value;const {items}=await parallax.accounts();account.replaceChildren();
  for(const a of items){const o=node('option',a.label);o.value=a.id;account.append(o);}
  if(items.some(a=>a.id===previous))account.value=previous;
  el('price-history-fetch').disabled=fetching||!account.value||!el('price-history-item').value;await history();
}
account.onchange=()=>{el('price-history-result').replaceChildren();history().catch(e=>el('price-history-feedback').textContent=e.message);};
el('price-history-form').onsubmit=async event=>{
  event.preventDefault();if(!account.value)return;fetching=true;account.disabled=true;el('price-history-fetch').disabled=true;
  el('price-history-feedback').textContent='正在读取价格历史，按 UTC 自然日研究；不会创建交易或刷新预算报价。';
  const id=account.value;
  try {
    const snapshot=await parallax.fetchPriceHistory(id,{scan_id:el('price-history-scan').value,item_key:el('price-history-item').value,window_days:Number(el('price-history-days').value),slippage_bps:Number(el('price-history-slippage').value)});
    if(account.value===id)render(snapshot);el('price-history-feedback').textContent='研究快照已保存，未加入预算或交易策略。';
    try{await history();}catch(e){el('price-history-feedback').textContent=`快照已保存，历史列表刷新失败：${e.message}`;}
  }catch(e){el('price-history-feedback').textContent=`本次未保存新快照：${e.message}；输入及原结果保留。`;}
  finally{fetching=false;account.disabled=false;el('price-history-fetch').disabled=!account.value||!el('price-history-item').value;}
};
window.addEventListener('parallax:account-created',()=>loadAccounts().catch(e=>el('price-history-feedback').textContent=e.message));
loadAccounts().catch(e=>el('price-history-feedback').textContent=e.message);
