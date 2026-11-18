import {parallax} from './lib/api.js';
import {rNode,renderResearchBanner,renderKpiGrid,renderDepthCard,syncScanSelectOptions} from './lib/research-view.js';

const el=id=>document.getElementById(id);
const account=el('buff-quote-account');
const scanSelect=el('buff-quote-scan-select');
let fetching=false;
let activeReport=null;
let scanList=[];
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const money=n=>`¥${(n/100).toFixed(2)}`;
const accountLabel=()=>account.selectedOptions?.[0]?.textContent||'当前账户';

async function refreshScanOptions(reportOverride=activeReport) {
  try {
    const {items}=await parallax.scans();
    scanList=items||[];
    syncScanSelectOptions(scanSelect,scanList,reportOverride,'BUFF');
  } catch(_) {
    syncScanSelectOptions(scanSelect,scanList,reportOverride,'BUFF');
  }
}

export function renderBuffCandidates(report) {
  activeReport=report;
  const selected=el('buff-quote-item').value;
  el('buff-quote-scan').value=report?.id||'';
  if(report?.id){
    const idx=scanList.findIndex(s=>s.id===report.id);
    if(idx>=0)scanList[idx]=report;
    else refreshScanOptions(report);
  }
  syncScanSelectOptions(scanSelect,scanList,report,'BUFF');
  el('buff-quote-item').replaceChildren();
  if(report?.status==='completed'&&report.request?.source==='steamdt') {
    for(const {item} of report.candidates.filter(c=>c.item.goods_id)) {
      const label=item.name&&item.name!==item.key?`${item.name} · ${item.key}`:item.key;
      const option=node('option',label);option.value=item.key;el('buff-quote-item').append(option);
    }
  }
  if([...el('buff-quote-item').options].some(o=>o.value===selected))el('buff-quote-item').value=selected;
  el('buff-quote-fetch').disabled=fetching||!el('buff-quote-item').value;
  if(report?.request?.source==='demo') {
    const hasReal=scanList.some(s=>s.request?.source==='steamdt'&&s.status==='completed');
    el('buff-quote-feedback').textContent=hasReal
      ?'当前选品为演示样本（不可查 BUFF）；可在左侧「来源选品扫描」直接切换已有 SteamDT 真实榜单，或在「选品工作台」执行新扫描。'
      :'当前为演示扫描，禁止向 BUFF 请求这些虚构商品；请先在「选品工作台」选择 SteamDT 真实榜单扫描。';
  } else if(report?.request?.source==='steamdt') {
    el('buff-quote-feedback').textContent=el('buff-quote-item').value
      ?'已关联 SteamDT 真实候选，请选择饰品后读取并保存首屏现货卖盘。'
      :'该次 SteamDT 扫描暂无带 BUFF 映射的可用候选。';
  }
}

export function readBuffQuoteInputs() {
  const ids=el('buff-snapshot-ids').value.split(/[,，]/).map(s=>s.trim()).filter(Boolean);
  return {buff_snapshot_ids:ids,buff_account_id:ids.length?account.value:null};
}

function render(quote) {
  const box=el('buff-quote-result');
  const use=node('button','将此报价加入预算引用');
  use.type='button';
  use.className='btn btn-sm btn-primary';
  use.onclick=()=>{
    if(account.value!==quote.account_id){el('buff-quote-feedback').textContent='请先选择这份快照所属账户';return;}
    const {buff_snapshot_ids:ids}=readBuffQuoteInputs();if(!ids.includes(quote.id))ids.push(quote.id);
    el('buff-snapshot-ids').value=ids.join(', ');
    el('buff-quote-feedback').textContent='已填写引用；生成计划时还会校验当前账户、代次、商品与时效。';
    window.dispatchEvent(new CustomEvent('parallax:quote-referenced',{detail:{source:'BUFF'}}));
    location.hash='planning';
  };

  const banner=renderResearchBanner(
    quote.request.item_key,
    `${accountLabel()} · 抓取于 ${new Date(quote.observed_at*1000).toLocaleString()}`,
    [use],
  );
  const totalVisible=(quote.page?.levels||[]).reduce((s,l)=>s+Number(l.quantity||0),0);
  const kpis=renderKpiGrid([
    {label:'首屏最低现货价',value:quote.lowest_cash_cents===null?'空卖盘':money(quote.lowest_cash_cents),tone:'good'},
    {label:'最低价可见条数',value:`${quote.lowest_tier_visible_units} 条`},
    {label:'首屏可见总卖单',value:`${totalVisible} 条`,hint:'未锁定库存，仅验证可买深度'},
  ]);
  const depthCard=renderDepthCard('BUFF 首屏现货卖单前 10 档','ask',quote.page?.levels||[],p=>money(p),'首屏暂无可见现金卖单。');
  box.replaceChildren(banner,kpis,depthCard);
}

async function history() {
  el('buff-quote-history').replaceChildren();if(!account.value)return;
  const {items}=await parallax.buffQuotes(account.value);
  for(const quote of items) {
    const button=node('button');
    button.type='button';button.className='history-row';
    button.append(
      node('strong',quote.request.item_key),
      node('span',`${quote.lowest_cash_cents!=null?money(quote.lowest_cash_cents):'空盘'} · ${new Date(quote.observed_at*1000).toLocaleDateString()}`),
    );
    button.onclick=async()=>{try{render(await parallax.buffQuote(account.value,quote.id));el('buff-quote-feedback').textContent='历史快照，未重新请求平台或确认登录态。';}catch(e){el('buff-quote-feedback').textContent=e.message;}};
    el('buff-quote-history').append(button);
  }
  if(!items.length)el('buff-quote-history').append(rNode('p','hint','该账户尚无 BUFF 报价快照。'));
  else if(!el('buff-quote-result').querySelector('.research-item-banner')){
    try{render(await parallax.buffQuote(account.value,items[0].id));}catch(_){}
  }
}

async function loadAccounts() {
  const preferred=account.value;const {items}=await parallax.accounts();account.replaceChildren();
  for(const a of items){const option=node('option',a.label);option.value=a.id;account.append(option);}
  if(items.some(a=>a.id===preferred))account.value=preferred;
  await history();
}

account.onchange=()=>{el('buff-quote-result').replaceChildren();history().catch(e=>el('buff-quote-feedback').textContent=e.message);};
el('buff-quote-form').onsubmit=async event=>{
  event.preventDefault();fetching=true;account.disabled=true;el('buff-quote-fetch').disabled=true;el('buff-quote-feedback').textContent='正在读取 BUFF 卖单…';
  try {
    if(!account.value)throw new Error('请先创建并选择账户');
    const quote=await parallax.fetchBuffQuote(account.value,{scan_id:el('buff-quote-scan').value,item_key:el('buff-quote-item').value});
    render(quote);el('buff-quote-feedback').textContent='快照已保存，未锁单或付款。';
    try{await history();}catch(e){el('buff-quote-feedback').textContent=`快照已保存，历史刷新失败：${e.message}`;}
  }catch(e){el('buff-quote-feedback').textContent=`读取未完成：${e.message}；输入和已显示结果保留。`;}
  finally{fetching=false;account.disabled=false;el('buff-quote-fetch').disabled=!el('buff-quote-item').value;}
};

scanSelect.onchange=async()=>{
  const id=scanSelect.value;
  if(!id)return;
  try{
    const report=await parallax.scan(id);
    renderBuffCandidates(report);
  }catch(e){el('buff-quote-feedback').textContent=`切换来源扫描失败：${e.message}`;}
};
window.addEventListener('parallax:account-created',()=>loadAccounts().catch(e=>el('buff-quote-feedback').textContent=e.message));
window.addEventListener('parallax:pagechange',({detail})=>{if(detail?.page==='buff-research')refreshScanOptions();});
refreshScanOptions();
loadAccounts().catch(e=>el('buff-quote-feedback').textContent=e.message);
