import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const account=el('price-history-account');
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
let fetching=false;
let inventoryOrigin=null;
const manual=()=>el('price-history-origin').value==='manual';
function readyHistory(){el('price-history-fetch').disabled=fetching||!account.value||!(manual()?el('price-history-manual').value.trim():el('price-history-item').value);}
function originFields(){
  el('price-history-manual').disabled=!manual();el('price-history-item').disabled=manual();
  el('price-history-manual').closest('label').hidden=!manual();el('price-history-scan').closest('label').hidden=manual();el('price-history-item').closest('label').hidden=manual();readyHistory();
}
el('price-history-origin').onchange=originFields;
el('price-history-manual').oninput=readyHistory;

const day=d=>new Date(d*86400000).toISOString().slice(0,10);
const money=m=>m?`${m.currency} ${(m.minor/100).toFixed(2)}`:'未知';
export function renderHistoryCandidates(report) {
  if(fetching)return;
  inventoryOrigin=null;
  const previous=el('price-history-item').value;el('price-history-scan').value=report.id;
  el('price-history-item').replaceChildren();
  if(report.status==='completed'&&report.request.source==='steamdt') {
    for(const {item} of report.candidates){const o=node('option',item.key);o.value=item.key;el('price-history-item').append(o);}
  }
  if([...el('price-history-item').options].some(o=>o.value===previous))el('price-history-item').value=previous;
  readyHistory();
  if(report.request.source==='demo')el('price-history-feedback').textContent='演示扫描禁止向 Steam 请求虚构商品的历史。';
}
function render(snapshot) {
  const a=snapshot.analysis,s=snapshot.series,r=a.risk;
  const status={unknown_currency:'币种含糊或单位未适配',no_usable_daily_prices:'没有可用日价格',insufficient_pairs:'缺少七自然日配对',historical_pairs_available:'已计算历史配对'};
  const box=el('price-history-result');box.replaceChildren(node('h3',snapshot.request.item_key),
    node('p',snapshot.request.manual_item?'来源：指定市场名称 · 按请求参数绑定，不是库存或资产证明':snapshot.request.inventory_snapshot_id?`来源：库存快照 ${snapshot.request.inventory_snapshot_id} · 商品级市场历史，不是该资产成交或成本记录`:`来源：SteamDT 扫描 ${snapshot.request.scan_id}`),
    node('p',`账户 ${snapshot.account_id} · 快照 ${snapshot.id} · 读取于 ${new Date(snapshot.observed_at*1000).toLocaleString()}`),
    node('p',`${status[a.status]??a.status} · 币种 ${s.currency??'未知'} · UTC 窗口 ${day(s.from_utc_day)} 至 ${day(s.until_utc_day_exclusive)}（不含结束日）`),
    node('p',`响应 ${s.response_rows} 条 · 窗口内 ${s.points.length} 条 · 排除窗口前 ${s.excluded_before_window} 条、当天 ${s.excluded_open_day} 条 · 缺少 ${a.missing_calendar_days} 个日期`),
    node('p',`七日配对 ${r?.sample_count??0} 个 · P10 ${r?.return_p10_bps==null?'未知':(r.return_p10_bps/100).toFixed(2)+'%'} · 参考历史日 ${a.reference_utc_day==null?'未知':day(a.reference_utc_day)} · 压力估算价 ${money(r?.conservative_gross)}`));
  for(const warning of a.warnings)box.append(node('p',warning));
  if(a.below_minor_unit_days)box.append(node('p',`${a.below_minor_unit_days} 个日期价格低于已适配最小单位，未当作零价。`));
  const metrics=a.stability,pct=value=>value==null?'未知':`${(value/100).toFixed(2)}%`;
  if(metrics){
    const section=node('section');section.append(node('h3','波动与自然日趋势'),
      node('p',`${metrics.algorithm} · 有效日 ${metrics.usable_days} · 缺失或价格不可用 ${metrics.missing_or_unusable_days} 日`),
      node('p',`样本 CV：原日价 ${pct(metrics.raw_sample_cv_bps)} / IQR 清洗后 ${pct(metrics.clean_sample_cv_bps)}；清洗均价 ${money(metrics.clean_mean)} · 中位数 ${money(metrics.clean_median)}`),
      node('p',`EMA7 ${money(metrics.ema7.ema)}（${metrics.ema7.usable_days}/7 个有效自然日） · EMA30 ${money(metrics.ema30.ema)}（${metrics.ema30.usable_days}/30）`),
      node('p',`近七自然日趋势：${metrics.trend7.usable_days} 个日价点 · 相对斜率 ${pct(metrics.trend7.relative_slope_bps_per_day)}/日 · R² ${metrics.trend7.r_squared_bps==null?'未定义／样本不足':(metrics.trend7.r_squared_bps/10000).toFixed(4)}`),
      node('p',`最后历史价的清洗区间位置 ${pct(metrics.reference_range_position_bps)}；为最低/最高价间位置，不是经验概率分位或实时价格。`));
    const detail=node('details');detail.append(node('summary',`IQR 证据 · 保留 ${metrics.iqr.retained_days} 日 / 排除 ${metrics.iqr.excluded_utc_days.length} 日`),
      node('p',metrics.iqr.method),node('p',metrics.iqr.excluded_utc_days.length?`仅清洗统计排除日期：${metrics.iqr.excluded_utc_days.map(day).join('、')}。原日线、EMA、趋势和 P10 不删除这些数据。`:'没有因 IQR 排除日价。'));
    section.append(detail);for(const warning of metrics.warnings)section.append(node('p',warning));box.append(section);
  }else box.append(node('p','此快照未包含稳定性指标（旧版本或币种未知）；不会在回看时静默重算。'));
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
  readyHistory();await history();
}
account.onchange=()=>{
  if(inventoryOrigin&&inventoryOrigin.account_id!==account.value){inventoryOrigin=null;el('price-history-scan').value='';el('price-history-item').replaceChildren();el('price-history-fetch').disabled=true;}
  readyHistory();el('price-history-result').replaceChildren();history().catch(e=>el('price-history-feedback').textContent=e.message);
};
window.addEventListener('parallax:inventory-history',async({detail})=>{
  if(fetching){el('inventory-feedback').textContent='价格历史正在读取，请完成后再选择库存商品。';return;}
  try{
    await loadAccounts();
    if(![...account.options].some(o=>o.value===detail.account_id))throw new Error('库存所属账户已不存在');
    account.value=detail.account_id;inventoryOrigin=detail;el('price-history-origin').value='bound';originFields();
    el('price-history-scan').value=detail.inventory_snapshot_id;
    const option=node('option',detail.item_key);option.value=detail.item_key;el('price-history-item').replaceChildren(option);
    el('price-history-result').replaceChildren();el('price-history-fetch').disabled=false;
    el('price-history-feedback').textContent='已选择库存快照中的商品；点击读取才请求 Steam，不证明当前仍持有，也不加入采购策略。';
    location.hash='price-history';await history();
  }catch(e){el('inventory-feedback').textContent=`选择历史研究失败：${e.message}`;}
});
el('price-history-form').onsubmit=async event=>{
  event.preventDefault();if(!account.value)return;fetching=true;account.disabled=true;el('price-history-fetch').disabled=true;
  el('price-history-feedback').textContent='正在读取价格历史，按 UTC 自然日研究；不会创建交易或刷新预算报价。';
  const id=account.value;
  try {
    const origin=manual()?{manual_item:true}:inventoryOrigin?{inventory_snapshot_id:inventoryOrigin.inventory_snapshot_id}:{scan_id:el('price-history-scan').value};
    const snapshot=await parallax.fetchPriceHistory(id,{...origin,item_key:manual()?el('price-history-manual').value.trim():el('price-history-item').value,window_days:Number(el('price-history-days').value),slippage_bps:Number(el('price-history-slippage').value)});
    if(account.value===id)render(snapshot);el('price-history-feedback').textContent='研究快照已保存，未加入预算或交易策略。';
    try{await history();}catch(e){el('price-history-feedback').textContent=`快照已保存，历史列表刷新失败：${e.message}`;}
  }catch(e){el('price-history-feedback').textContent=`本次未保存新快照：${e.message}；输入及原结果保留。`;}
  finally{fetching=false;account.disabled=false;readyHistory();}
};
originFields();
window.addEventListener('parallax:account-created',()=>loadAccounts().catch(e=>el('price-history-feedback').textContent=e.message));
loadAccounts().catch(e=>el('price-history-feedback').textContent=e.message);
