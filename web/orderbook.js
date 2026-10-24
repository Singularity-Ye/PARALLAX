import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const native=(minor,currency)=>new Intl.NumberFormat('zh-CN',{style:'currency',currency}).format(minor/100);
let selectedBookId=null;
let selectedBookCurrency=null;
let pricingBusy=false;
let valuationRevision=0;
async function updateValuation(){
  if(!selectedBookId)return;
  const id=selectedBookId,currency=el('book-display-currency').value,revision=++valuationRevision;
  el('book-valuation-result').replaceChildren();el('book-valuation-feedback').textContent='正在换算显示金额；保留原盘口与时间…';
  try{
    const value=await parallax.orderBookValuation(id,{display_currency:currency});
    if(revision!==valuationRevision)return;
    el('book-valuation-result').append(node('p',`折合 ${currency}：最低卖价 ${value.best_ask?native(value.best_ask.minor,currency):'未知'} · 最高买价 ${value.best_bid?native(value.best_bid.minor,currency):'未知'}；原币 ${value.native_currency}。`));
    el('book-valuation-feedback').textContent=value.fx?`参考汇率 ${value.fx.id} · 报价日期 ${[...new Set(Object.values(value.fx.rates).map(r=>r.date))].join(' / ')} · 原盘口时间未刷新。`:'同币种，无需换汇；原盘口时间未刷新。';
  }catch(e){if(revision===valuationRevision)el('book-valuation-feedback').textContent=`换算未完成：${e.message}；原币行情仍可查看。`;}
}
el('book-display-currency').onchange=()=>{el('listing-preview-result').replaceChildren();updateValuation();};
export function readSteamSnapshotIds() {
  return el('steam-snapshot-ids').value.split(/[,，]/).map(s=>s.trim()).filter(Boolean);
}
function render(book) {
  selectedBookId=book.id;
  selectedBookCurrency=book.book.currency;
  el('listing-snapshot').value=`${book.request.market_hash_name} · ${book.book.currency} · ${book.id}`;
  el('listing-preview-create').disabled=pricingBusy;
  el('listing-fees-enabled').checked=false;el('listing-fees').disabled=true;
  el('listing-fees').querySelector('legend').textContent=`费用模型 · ${book.book.currency} 最小单位（当前均为百分之一）`;
  el('listing-preview-result').replaceChildren();
  el('listing-preview-feedback').textContent='已选中；异币使用有效参考汇率比较人民币策略阈值，不刷新盘口时间。费用模型如启用须按原币填写。';
  updateValuation();
  const box=el('book-result'), {summary:s,book:depth}=book;
  box.replaceChildren(node('h3',`${book.request.market_hash_name} · ${depth.currency}`),
    node('p',`快照 ${book.id} · 请求 ${book.requested_currency} / 实际返回 ${depth.currency} · 抓取于 ${new Date(book.observed_at*1000).toLocaleString()}`),
    node('p',`最低卖价 ${s.best_ask?native(s.best_ask.minor,depth.currency):'无卖单'} · 最高买价 ${s.best_bid?native(s.best_bid.minor,depth.currency):'无买单'} · 毛价差 ${s.spread_minor===null?'未知':native(s.spread_minor,depth.currency)}`),
    node('p',`返回档位的数量合计：卖盘 ${s.visible_ask_quantity} / 买盘 ${s.visible_bid_quantity}；不是日成交量，也不是 BUFF 可购买数量。`));
  const warnings=node('ul');for(const warning of book.warnings)warnings.append(node('li',warning==='平台实际返回非 CNY；保留原币，不换汇，不能用于人民币预算'?'平台实际返回非 CNY；保留原币，当前显示与预算通过独立有效汇率快照换算。':warning));box.append(warnings);
  for(const [label,levels] of [['卖盘',depth.asks],['买盘',depth.bids]]) {
    const details=node('details');details.append(node('summary',`${label} ${levels.length} 档（页面仅展示前十档）`));
    const rows=node('ul');for(const level of levels.slice(0,10))rows.append(node('li',`${native(level.price_minor,depth.currency)} · 数量 ${level.quantity}`));
    details.append(rows);box.append(details);
  }
  if(s.best_ask && (s.spread_minor===null || s.spread_minor>=0)) {
    const use=node('button','将此快照 ID 加入预算引用');use.type='button';use.className='secondary';
    use.onclick=()=>{
      const ids=readSteamSnapshotIds();if(!ids.includes(book.id))ids.push(book.id);
      el('steam-snapshot-ids').value=ids.join(', ');
      el('book-feedback').textContent='已填写引用 ID；预算会自动换算为人民币，仍核对商品、双方时效和汇率有效性。';
      window.dispatchEvent(new CustomEvent('parallax:quote-referenced',{detail:{source:'Steam'}}));
      location.hash='planning';
    };box.append(use);
  }
}
function renderListingPreview(preview) {
  const box=el('listing-preview-result'),a=preview.analysis;
  const reasons={empty_asks:'无卖盘',single_level:'分析范围只有一档',regular_undercut:'常规压价',gap_jump:'断层跳档'};
  box.replaceChildren(node('h3',`${preview.market.market_hash_name} · 挂牌参考`),
    node('p',`快照 ${preview.snapshot_id} · 抓取于 ${new Date(preview.observed_at*1000).toLocaleString()} · ${preview.freshness==='historical_only'?'仅历史研究':'所选时效内的快照研究'} · ${a.algorithm}`),
    node('p',`原币挂牌参考 ${a.native_gross_listing_estimate?native(a.native_gross_listing_estimate.minor,a.book_currency):'未知（无卖单）'} · 折合 ${preview.display_gross_listing_estimate?native(preview.display_gross_listing_estimate.minor,preview.display_currency):'未知'} · 净钱包估算 ${preview.display_net_estimate?native(preview.display_net_estimate.minor,preview.display_currency):'未知／无可负担正净额'} · ${reasons[a.reason]}`),
    node('p',`剔除最低档 ${a.ignored_lowest_tiers.length} 档 · ${a.stopped_by_blocking_volume?'阻挡量超限，停止继续找断层':'未因阻挡量停止'} · ${a.floor_applied?'已应用价格下限':'未触发价格下限'}`));
  if(a.wall)box.append(node('p',`墙分析截至原卖盘第 ${a.wall.level_index+1} 档，原币价 ${native(a.wall.price_minor,a.book_currency)}，剔除后累计 ${a.wall.cumulative_quantity} 件；${a.wall.reached_threshold?'达到所选墙量':'可见深度未达到墙量'}。`));
  if(preview.fx)box.append(node('p',`参考汇率 ${preview.fx.id} · 牌价日期 ${[...new Set(Object.values(preview.fx.rates).map(r=>r.date))].join(' / ')}；不是实际结算汇率。`));
  const trace=node('details');trace.append(node('summary','剔除与断层依据'));
  const rows=node('ul');
  for(const tier of a.ignored_lowest_tiers)rows.append(node('li',`剔除 ${native(tier.price_minor,a.book_currency)} · ${tier.quantity} 件`));
  for(const gap of a.gap_checks)rows.append(node('li',`${native(gap.lower_minor,a.book_currency)} → ${native(gap.upper_minor,a.book_currency)} · 阻挡量 ${gap.blocking_quantity} · 阈值取 ${native(gap.absolute_threshold_minor,'CNY')} 与 ${gap.relative_threshold_bps/100}% 中较高者 · ${gap.jumped?'严格超过，跳档':'未严格超过，不跳档'}`));
  if(!rows.children.length)rows.append(node('li','没有剔除或可检查的相邻断层。'));
  trace.append(rows);box.append(trace);
  const warnings=node('ul');for(const warning of preview.warnings)warnings.append(node('li',warning));box.append(warnings);
}
el('listing-fees-enabled').onchange=event=>{el('listing-fees').disabled=!event.target.checked;};
el('listing-preview-form').onsubmit=async event=>{
  event.preventDefault();if(!selectedBookId||pricingBusy)return;
  const id=selectedBookId,display_currency=el('book-display-currency').value,data=new FormData(event.target),integer=name=>Number(data.get(name));
  const policy={currency:'CNY'};
  for(const key of ['wall_volume_threshold','max_ignore_volume','min_lowest_tier_volume','min_step_minor','min_floor_minor','offset_minor'])policy[key]=integer(key);
  let fee_schedule=null;
  if(data.has('estimate_fees')) {
    fee_schedule={currency:selectedBookCurrency};
    for(const key of ['platform_bps','publisher_bps','platform_min_minor','publisher_min_minor','increment_minor'])fee_schedule[key]=integer(key);
  }
  pricingBusy=true;el('listing-preview-create').disabled=true;el('listing-preview-feedback').textContent='正在使用本地快照计算…';
  try {
    const preview=await parallax.listingPricePreview(id,{policy,display_currency,max_quote_age_secs:integer('max_quote_age_secs'),fee_schedule});
    if(selectedBookId!==id||el('book-display-currency').value!==display_currency)return;
    renderListingPreview(preview);el('listing-preview-feedback').textContent='本地研究计算完成；未创建订单、上架或修改预算。';
  } catch(e){if(selectedBookId===id)el('listing-preview-feedback').textContent=`计算失败：${e.message}；输入与上次结果保留。`;}
  finally {pricingBusy=false;el('listing-preview-create').disabled=!selectedBookId;}
};
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
    const data=new FormData(event.target);
    const book=await parallax.fetchOrderBook({app_id:730,market_hash_name:data.get('market_hash_name'),currency:data.get('currency')});
    render(book);el('book-feedback').textContent='盘口快照已保存；请查看实际币种与限制。';
    try {await history();} catch(e){el('book-feedback').textContent=`快照已保存，历史列表刷新失败：${e.message}`;}
  } catch(e){el('book-feedback').textContent=`读取失败：${e.message}；输入与上次结果保留。`;}
  finally {el('book-fetch').disabled=false;}
};
history().catch(e=>el('book-feedback').textContent=`盘口历史读取失败：${e.message}`);
