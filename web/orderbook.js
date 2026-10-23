import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const native=(minor,currency)=>new Intl.NumberFormat('zh-CN',{style:'currency',currency}).format(minor/100);
export function readSteamSnapshotIds() {
  return el('steam-snapshot-ids').value.split(/[,，]/).map(s=>s.trim()).filter(Boolean);
}
function render(book) {
  const box=el('book-result'), {summary:s,book:depth}=book;
  box.replaceChildren(node('h3',`${book.request.market_hash_name} · ${depth.currency}`),
    node('p',`快照 ${book.id} · 抓取于 ${new Date(book.observed_at*1000).toLocaleString()}`),
    node('p',`最低卖价 ${s.best_ask?native(s.best_ask.minor,depth.currency):'无卖单'} · 最高买价 ${s.best_bid?native(s.best_bid.minor,depth.currency):'无买单'} · 毛价差 ${s.spread_minor===null?'未知':native(s.spread_minor,depth.currency)}`),
    node('p',`返回档位的数量合计：卖盘 ${s.visible_ask_quantity} / 买盘 ${s.visible_bid_quantity}；不是日成交量，也不是 BUFF 可购买数量。`));
  const warnings=node('ul');for(const warning of book.warnings)warnings.append(node('li',warning));box.append(warnings);
  for(const [label,levels] of [['卖盘',depth.asks],['买盘',depth.bids]]) {
    const details=node('details');details.append(node('summary',`${label} ${levels.length} 档（页面仅展示前十档）`));
    const rows=node('ul');for(const level of levels.slice(0,10))rows.append(node('li',`${native(level.price_minor,depth.currency)} · 数量 ${level.quantity}`));
    details.append(rows);box.append(details);
  }
  if(depth.currency==='CNY' && s.best_ask && (s.spread_minor===null || s.spread_minor>=0)) {
    const use=node('button','将此快照 ID 加入预算引用');use.type='button';use.className='secondary';
    use.onclick=()=>{
      const ids=readSteamSnapshotIds();if(!ids.includes(book.id))ids.push(book.id);
      el('steam-snapshot-ids').value=ids.join(', ');
      el('book-feedback').textContent='已填写引用 ID；生成计划时后端仍会核对商品、币种和双方报价有效期。';
    };box.append(use);
  }
}
async function history() {
  const {items}=await parallax.orderBooks();
  el('book-history').replaceChildren(...items.map(book=>{
    const button=node('button',`${book.request.market_hash_name} · ${book.book.currency} · ${new Date(book.observed_at*1000).toLocaleString()}`);
    button.type='button';button.className='history-row';button.onclick=async()=>{
      try {render(await parallax.orderBook(book.id));el('book-feedback').textContent='已读取历史快照，未刷新平台报价。';}
      catch(e){el('book-feedback').textContent=e.message;}
    };return button;
  }));
  if(!items.length)el('book-history').append(node('p','尚无盘口快照。'));
}
el('order-book-form').onsubmit=async event=>{
  event.preventDefault();el('book-fetch').disabled=true;el('book-feedback').textContent='正在读取公开盘口…';
  try {
    const book=await parallax.fetchOrderBook({app_id:730,market_hash_name:new FormData(event.target).get('market_hash_name')});
    render(book);el('book-feedback').textContent='盘口快照已保存；请查看实际币种与限制。';
    try {await history();} catch(e){el('book-feedback').textContent=`快照已保存，历史列表刷新失败：${e.message}`;}
  } catch(e){el('book-feedback').textContent=`读取失败：${e.message}；输入与上次结果保留。`;}
  finally {el('book-fetch').disabled=false;}
};
history().catch(e=>el('book-feedback').textContent=`盘口历史读取失败：${e.message}`);
