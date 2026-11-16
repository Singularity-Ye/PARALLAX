import {parallax} from './lib/api.js';
import {rNode,renderResearchBanner,renderKpiGrid,renderPriceHistoryChart,syncScanSelectOptions} from './lib/research-view.js';

const el=id=>document.getElementById(id);
const account=el('price-history-account');
const scanSelect=el('price-history-scan-select');
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
let fetching=false;
let inventoryOrigin=null;
let activeReport=null;
let scanList=[];

const manual=()=>el('price-history-origin').value==='manual';
function readyHistory(){el('price-history-fetch').disabled=fetching||!account.value||!(manual()?el('price-history-manual').value.trim():el('price-history-item').value);}
function syncSourceDropdown(){
  const invOpt=inventoryOrigin?{value:'__inventory__',label:`库存快照 · ${inventoryOrigin.item_key}`}:null;
  syncScanSelectOptions(scanSelect,scanList,activeReport,'Steam 历史',invOpt);
}
async function refreshScanOptions(){
  try{
    const {items}=await parallax.scans();
    scanList=items||[];
    syncSourceDropdown();
  }catch(_){
    syncSourceDropdown();
  }
}
function originFields(){
  el('price-history-manual').disabled=!manual();
  el('price-history-item').disabled=manual();
  scanSelect.disabled=manual();
  el('price-history-manual').closest('label').hidden=!manual();
  scanSelect.closest('label').hidden=manual();
  el('price-history-item').closest('label').hidden=manual();
  readyHistory();
}
el('price-history-origin').onchange=originFields;
el('price-history-manual').oninput=readyHistory;

const day=d=>new Date(d*86400000).toISOString().slice(0,10);
const money=m=>m?`${m.currency} ${(m.minor/100).toFixed(2)}`:'未知';

export function renderHistoryCandidates(report) {
  if(fetching)return;
  inventoryOrigin=null;
  activeReport=report;
  const previous=el('price-history-item').value;el('price-history-scan').value=report?.id||'';
  syncSourceDropdown();
  if(report?.id&&!scanList.some(s=>s.id===report.id)) refreshScanOptions();
  el('price-history-item').replaceChildren();
  if(report?.status==='completed'&&report.request?.source==='steamdt') {
    for(const {item} of report.candidates){
      const label=item.name&&item.name!==item.key?`${item.name} · ${item.key}`:item.key;
      const o=node('option',label);o.value=item.key;el('price-history-item').append(o);
    }
  }
  if([...el('price-history-item').options].some(o=>o.value===previous))el('price-history-item').value=previous;
  readyHistory();
  if(report?.request?.source==='demo'){
    const hasReal=scanList.some(s=>s.request?.source==='steamdt'&&s.status==='completed');
    el('price-history-feedback').textContent=hasReal
      ?'当前选品为演示样本（不可查 Steam）；可从左侧「关联来源」切换已有 SteamDT 真实榜单，或点击下方「快捷填入真实商品」。'
      :'演示扫描禁止向 Steam 请求虚构商品的历史；可切换「指定市场名称」直接研究真实 CS2 饰品。';
  }
}

function render(snapshot) {
  const a=snapshot.analysis,s=snapshot.series,r=a.risk;
  const status={unknown_currency:'币种含糊或单位未适配',no_usable_daily_prices:'没有可用日价格',insufficient_pairs:'缺少七自然日配对',historical_pairs_available:'已计算历史配对'};
  const originText=snapshot.request.manual_item
    ?'指定市场名称'
    :snapshot.request.inventory_snapshot_id
      ?'库存快照'
      :'SteamDT 选品扫描';

  const watchButton=node('button','加入关注列表');
  watchButton.type='button';
  watchButton.className='btn btn-sm btn-secondary';
  watchButton.onclick=()=>window.dispatchEvent(new CustomEvent('parallax:watch-item',{detail:{account_id:snapshot.account_id,item_key:snapshot.request.item_key}}));

  const banner=renderResearchBanner(
    snapshot.request.item_key,
    `${originText} · ${status[a.status]??a.status} · 窗口 ${day(s.from_utc_day)} 至 ${day(s.until_utc_day_exclusive)} · 读取于 ${new Date(snapshot.observed_at*1000).toLocaleString()}`,
    [watchButton],
  );

  const metrics=a.stability,pct=value=>value==null?'未知':`${(value/100).toFixed(2)}%`;
  const kpiList=[
    {label:'7 日配对 P10 回撤',value:r?.return_p10_bps==null?'未知':`${(r.return_p10_bps/100).toFixed(2)}%`,hint:`有效配对 ${r?.sample_count??0} 组 · 压力价 ${money(r?.conservative_gross)}`,tone:'good'},
    {label:'UTC 样本覆盖',value:`${s.points.length} 条`,hint:`缺失 ${a.missing_calendar_days} 日 · 原始响应 ${s.response_rows} 条`},
  ];
  if(metrics) {
    kpiList.push(
      {label:'IQR 清洗均价 / 中位',value:money(metrics.clean_mean),hint:`中位数 ${money(metrics.clean_median)}`},
      {label:'EMA7 / EMA30 均线',value:money(metrics.ema7.ema),hint:`EMA30 ${money(metrics.ema30.ema)} (${metrics.ema30.usable_days}/30日)`},
      {label:'波动率 CV (原 / 清洗)',value:pct(metrics.clean_sample_cv_bps),hint:`原日价 CV ${pct(metrics.raw_sample_cv_bps)}`},
      {label:'近 7 日斜率 / R²',value:metrics.trend7.relative_slope_bps_per_day==null?'未知':`${pct(metrics.trend7.relative_slope_bps_per_day)}/日`,hint:`R² ${metrics.trend7.r_squared_bps==null?'样本不足':(metrics.trend7.r_squared_bps/10000).toFixed(4)}`},
    );
  }

  const chartCard=renderPriceHistoryChart(a.daily_bars,metrics?.clean_mean?.minor,money);

  const barsCard=rNode('div','research-panel-card');
  const barsHead=rNode('div','research-card-head');
  barsHead.append(rNode('h4','','近期 UTC 自然日成交序列'),rNode('span','badge muted',`展示最近 ${Math.min(14,a.daily_bars.length)} / ${a.daily_bars.length} 日`));
  barsCard.append(barsHead);
  if(a.daily_bars.length) {
    const tbl=rNode('table','cockpit-table');
    tbl.innerHTML='<thead><tr><th>UTC 日期</th><th style="text-align:right">日均 VWAP</th><th style="text-align:right">已知成交量</th><th style="text-align:right">零量 / 缺量样本</th></tr></thead>';
    const tbody=document.createElement('tbody');
    for(const b of [...a.daily_bars].slice(-14).reverse()) {
      const tr=document.createElement('tr');
      const tdDate=rNode('td','col-mono',day(b.utc_day));
      const tdVwap=rNode('td','col-mono',money(b.vwap));tdVwap.style.textAlign='right';
      const tdVol=rNode('td','col-mono',`${b.volume}`);tdVol.style.textAlign='right';
      const tdAnom=rNode('td','col-mono',`${b.zero_volume_samples} / ${b.missing_volume_samples}`);tdAnom.style.textAlign='right';
      tr.append(tdDate,tdVwap,tdVol,tdAnom);
      tbody.append(tr);
    }
    tbl.append(tbody);
    barsCard.append(tbl);
  } else {
    barsCard.append(rNode('p','hint','当前窗口内无有效日线记录。'));
  }

  const box=el('price-history-result');
  box.replaceChildren(banner,renderKpiGrid(kpiList),chartCard,barsCard);

  if(metrics?.iqr) {
    const detail=node('details');
    detail.append(node('summary',`IQR 离群清洗证据 · 保留 ${metrics.iqr.retained_days} 日 / 排除 ${metrics.iqr.excluded_utc_days.length} 日`));
    detail.append(node('p',metrics.iqr.method),node('p',metrics.iqr.excluded_utc_days.length?`清洗统计排除日期：${metrics.iqr.excluded_utc_days.map(day).join('、')}（原日线/EMA/P10 仍保留原样本）。`:'没有因 IQR 排除的离群日期。'));
    box.append(detail);
  }
  if(a.warnings?.length) {
    const warnDet=node('details');
    warnDet.append(node('summary',`风控边界说明 (${a.warnings.length} 条)`));
    const ul=node('ul');a.warnings.forEach(w=>ul.append(node('li',w)));
    warnDet.append(ul);box.append(warnDet);
  }
}

async function history() {
  const id=account.value;el('price-history-saved').replaceChildren();if(!id)return;
  const {items}=await parallax.priceHistories(id);if(account.value!==id)return;
  for(const snapshot of items) {
    const b=node('button');b.type='button';b.className='history-row';
    b.append(node('strong',snapshot.request.item_key),node('span',new Date(snapshot.observed_at*1000).toLocaleDateString()));
    b.onclick=async()=>{try{const saved=await parallax.priceHistory(id,snapshot.id);if(account.value!==id)return;render(saved);el('price-history-feedback').textContent='保存时的历史研究结果，未重新请求或计算当前行情。';}catch(e){if(account.value===id)el('price-history-feedback').textContent=e.message;}};
    el('price-history-saved').append(b);
  }
  if(!items.length)el('price-history-saved').append(rNode('p','hint','该账户尚无价格历史研究快照。'));
  else if(!el('price-history-result').querySelector('.research-item-banner')){
    try{const saved=await parallax.priceHistory(id,items[0].id);if(account.value===id)render(saved);}catch(_){}
  }
}

async function loadAccounts() {
  const previous=account.value;const {items}=await parallax.accounts();account.replaceChildren();
  for(const a of items){const o=node('option',a.label);o.value=a.id;account.append(o);}
  if(items.some(a=>a.id===previous))account.value=previous;
  readyHistory();await history();
}

account.onchange=()=>{
  if(inventoryOrigin&&inventoryOrigin.account_id!==account.value){inventoryOrigin=null;el('price-history-scan').value='';el('price-history-item').replaceChildren();syncSourceDropdown();el('price-history-fetch').disabled=true;}
  readyHistory();el('price-history-result').replaceChildren();history().catch(e=>el('price-history-feedback').textContent=e.message);
};

scanSelect.onchange=async()=>{
  const val=scanSelect.value;
  if(!val||val==='__inventory__')return;
  try{
    const report=await parallax.scan(val);
    renderHistoryCandidates(report);
  }catch(e){el('price-history-feedback').textContent=`切换关联扫描失败：${e.message}`;}
};

window.addEventListener('parallax:inventory-history',async({detail})=>{
  if(fetching){el('inventory-feedback').textContent='价格历史正在读取，请完成后再选择库存商品。';return;}
  try{
    await loadAccounts();
    if(![...account.options].some(o=>o.value===detail.account_id))throw new Error('库存所属账户已不存在');
    account.value=detail.account_id;inventoryOrigin=detail;el('price-history-origin').value='bound';syncSourceDropdown();originFields();
    el('price-history-scan').value=detail.inventory_snapshot_id;
    const option=node('option',detail.item_key);option.value=detail.item_key;el('price-history-item').replaceChildren(option);
    el('price-history-result').replaceChildren();el('price-history-fetch').disabled=false;
    el('price-history-feedback').textContent='已选择库存快照中的商品；点击读取才请求 Steam，不证明当前仍持有，也不加入采购策略。';
    location.hash='price-history';await history();
  }catch(e){el('inventory-feedback').textContent=`选择历史研究失败：${e.message}`;}
});

window.addEventListener('parallax:scan-item-history',async({detail})=>{
  if(fetching)return;
  const scan=detail?.scan,item=detail?.item;
  if(!scan||!item?.key)return;
  el('price-history-origin').value='bound';
  renderHistoryCandidates(scan);
  originFields();
  if(![...el('price-history-item').options].some(o=>o.value===item.key)){
    const o=node('option',item.name&&item.name!==item.key?`${item.name} · ${item.key}`:item.key);
    o.value=item.key;el('price-history-item').append(o);
  }
  el('price-history-item').value=item.key;
  readyHistory();
  el('price-history-result').replaceChildren();
  let shownCached=false;
  if(account.value){
    try{
      const {items}=await parallax.priceHistories(account.value);
      const match=(items||[]).find(s=>s.request?.item_key===item.key);
      if(match){
        const saved=await parallax.priceHistory(account.value,match.id);
        render(saved);
        shownCached=true;
      }
    }catch(_){}
  }
  el('price-history-feedback').textContent=shownCached
    ?`已带入「${item.name||item.key}」并展示本地历史走势；点击「读取并保存历史研究」可按当前账户刷新至最新 UTC 日线。`
    :`已从选品工作台带入「${item.name||item.key}」；点击「读取并保存历史研究」即可直连 Steam 拉取 90 日量价走势。`;
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

document.querySelectorAll('.history-preset-btn').forEach(btn=>{
  btn.onclick=()=>{
    el('price-history-origin').value='manual';
    originFields();
    el('price-history-manual').value=btn.dataset.name||'';
    readyHistory();
  };
});

originFields();
refreshScanOptions();
window.addEventListener('parallax:pagechange',({detail})=>{if(detail?.page==='price-history')refreshScanOptions();});
window.addEventListener('parallax:account-created',()=>loadAccounts().catch(e=>el('price-history-feedback').textContent=e.message));
loadAccounts().catch(e=>el('price-history-feedback').textContent=e.message);

window.addEventListener('parallax:watch-history',async({detail})=>{
 if(fetching){el('strategy-watch-feedback').textContent='历史正在读取，请稍后重试。';return;}
 try{await loadAccounts();if(![...account.options].some(o=>o.value===detail.account_id))throw new Error('账户不存在');
 account.value=detail.account_id;inventoryOrigin=null;el('price-history-origin').value='manual';el('price-history-manual').value=detail.item_key;originFields();readyHistory();el('price-history-result').replaceChildren();el('price-history-feedback').textContent='已选择关注商品，点击读取才请求Steam。';location.hash='price-history';await history();}
 catch(e){el('strategy-watch-feedback').textContent=e.message;}
});

window.addEventListener('parallax:catalog-history',({detail})=>{
 if(fetching){document.getElementById('catalog-feedback').textContent='历史正在读取，请稍后再选择商品。';return;}
 inventoryOrigin=null;el('price-history-origin').value='manual';el('price-history-manual').value=detail.item_key;originFields();readyHistory();
 el('price-history-result').replaceChildren();el('price-history-feedback').textContent='已带入Steam检索的准确市场名，请选择账户并点击读取。尚未请求历史，不会加入预算。';location.hash='price-history';
});
