import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const account=el('buff-quote-account');
let fetching=false;
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const money=n=>`¥${(n/100).toFixed(2)}`;
export function renderBuffCandidates(report) {
  const selected=el('buff-quote-item').value;
  el('buff-quote-scan').value=report.id;
  el('buff-quote-item').replaceChildren();
  if(report.status==='completed'&&report.request.source==='steamdt') {
    for(const {item} of report.candidates.filter(c=>c.item.goods_id)) {
      const option=node('option',`${item.key} · goods ${item.goods_id}`);option.value=item.key;el('buff-quote-item').append(option);
    }
  }
  if([...el('buff-quote-item').options].some(o=>o.value===selected))el('buff-quote-item').value=selected;
  el('buff-quote-fetch').disabled=fetching||!el('buff-quote-item').value;
  if(report.request.source==='demo')el('buff-quote-feedback').textContent='当前为演示扫描，禁止向 BUFF 请求这些虚构商品。';
}
export function readBuffQuoteInputs() {
  const ids=el('buff-snapshot-ids').value.split(/[,，]/).map(s=>s.trim()).filter(Boolean);
  return {buff_snapshot_ids:ids,buff_account_id:ids.length?account.value:null};
}
function render(quote) {
  const box=el('buff-quote-result');
  box.replaceChildren(node('h3',quote.request.item_key),node('p',`账户 ${quote.account_id} · 快照 ${quote.id} · ${new Date(quote.observed_at*1000).toLocaleString()}`),
    node('p',`首屏最低价 ${quote.lowest_cash_cents===null?'无卖单':money(quote.lowest_cash_cents)} · 最低价可见卖单 ${quote.lowest_tier_visible_units} 条`),
    node('p','只保存首屏可见价格，未锁定数量；CNY 单位沿用待实测的原协议，不代表付款报价。'));
  const levels=node('ul');for(const level of quote.page.levels.slice(0,10))levels.append(node('li',`${money(level.price_minor)} · 可见 ${level.quantity} 条`));box.append(levels);
  const use=node('button','将此报价加入预算引用');use.type='button';use.className='secondary';use.onclick=()=>{
    if(account.value!==quote.account_id){el('buff-quote-feedback').textContent='请先选择这份快照所属账户';return;}
    const {buff_snapshot_ids:ids}=readBuffQuoteInputs();if(!ids.includes(quote.id))ids.push(quote.id);
    el('buff-snapshot-ids').value=ids.join(', ');
    el('buff-quote-feedback').textContent='已填写引用；生成计划时还会校验当前账户、代次、商品与时效。';
  };box.append(use);
}
async function history() {
  el('buff-quote-history').replaceChildren();if(!account.value)return;
  const {items}=await parallax.buffQuotes(account.value);
  for(const quote of items) {
    const button=node('button',`${quote.request.item_key} · ${new Date(quote.observed_at*1000).toLocaleString()}`);button.type='button';button.className='history-row';
    button.onclick=async()=>{try{render(await parallax.buffQuote(account.value,quote.id));el('buff-quote-feedback').textContent='历史快照，未重新请求平台或确认登录态。';}catch(e){el('buff-quote-feedback').textContent=e.message;}};
    el('buff-quote-history').append(button);
  }
  if(!items.length)el('buff-quote-history').append(node('p','该账户尚无报价。'));
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
window.addEventListener('parallax:account-created',()=>loadAccounts().catch(e=>el('buff-quote-feedback').textContent=e.message));
loadAccounts().catch(e=>el('buff-quote-feedback').textContent=e.message);
