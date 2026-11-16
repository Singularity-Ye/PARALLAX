import {parallax} from './lib/api.js';
import {currencyOptions} from './lib/currencies.js';
import {pricingEditor,feeEditor} from './lib/pricing-fields.js';
import {node,nativeMoney as native,pricingTrace} from './lib/pricing-view.js';
import {rNode,renderResearchBanner,renderKpiGrid,renderDepthCard} from './lib/research-view.js';

const el=id=>document.getElementById(id);
const priceFields=pricingEditor(el('listing-policy-fields'));
const feeFields=feeEditor(el('listing-fee-fields'));
for(const id of ['book-display-currency','book-request-currency'])currencyOptions(el(id));

let selectedBookId=null;
let selectedBookCurrency=null;
let pricingBusy=false;
let valuationRevision=0;

async function updateValuation(){
  if(!selectedBookId)return;
  const id=selectedBookId,currency=el('book-display-currency').value,revision=++valuationRevision;
  el('book-valuation-result').replaceChildren();
  el('book-valuation-feedback').textContent='正在换算显示金额；保留原盘口与时间…';
  try{
    const value=await parallax.orderBookValuation(id,{display_currency:currency});
    if(revision!==valuationRevision)return;
    const grid=renderKpiGrid([
      {label:`折合最低卖价 (${currency})`,value:value.best_ask?native(value.best_ask.minor,currency):'未知',tone:'good'},
      {label:`折合最高买价 (${currency})`,value:value.best_bid?native(value.best_bid.minor,currency):'未知'},
      {label:'原始盘口币种',value:value.native_currency},
    ]);
    el('book-valuation-result').replaceChildren(grid);
    el('book-valuation-feedback').textContent=value.fx?`参考汇率 ${value.fx.id} · 牌价日期 ${[...new Set(Object.values(value.fx.rates).map(r=>r.date))].join(' / ')} · 原盘口时间未刷新。`:'同币种，无需换汇；原盘口时间未刷新。';
  }catch(e){
    if(revision===valuationRevision)el('book-valuation-feedback').textContent=`换算未完成：${e.message}；原币行情仍可查看。`;
  }
}

el('book-display-currency').onchange=()=>{el('listing-preview-result').replaceChildren();updateValuation();};

export function readSteamSnapshotIds() {
  return el('steam-snapshot-ids').value.split(/[,，]/).map(s=>s.trim()).filter(Boolean);
}

function render(book) {
  selectedBookId=book.id;
  selectedBookCurrency=book.book.currency;
  feeFields.setCurrency(selectedBookCurrency);
  el('listing-snapshot').value=`${book.request.market_hash_name} · ${book.book.currency} · ${book.id.slice(0,8)}`;
  el('listing-preview-create').disabled=pricingBusy;
  el('listing-fees-enabled').checked=false;
  el('listing-fees').disabled=true;
  el('listing-fees').querySelector('legend').textContent=`费用模型 · ${book.book.currency} 最小单位（当前均为百分之一）`;
  el('listing-preview-result').replaceChildren();
  el('listing-preview-feedback').textContent='已选中；异币使用有效参考汇率比较人民币策略阈值，不刷新盘口时间。';
  updateValuation();

  const box=el('book-result'), {summary:s,book:depth}=book;
  const actions=[];
  if(s.best_ask && (s.spread_minor===null || s.spread_minor>=0)) {
    const use=node('button','将此快照 ID 加入预算引用');
    use.type='button';
    use.className='btn btn-sm btn-primary';
    use.onclick=()=>{
      const ids=readSteamSnapshotIds();
      if(!ids.includes(book.id))ids.push(book.id);
      el('steam-snapshot-ids').value=ids.join(', ');
      el('book-feedback').textContent='已填写引用 ID；预算会自动换算为人民币，仍核对商品、双方时效和汇率有效性。';
      window.dispatchEvent(new CustomEvent('parallax:quote-referenced',{detail:{source:'Steam'}}));
      location.hash='planning';
    };
    actions.push(use);
  }

  const banner=renderResearchBanner(
    book.request.market_hash_name,
    `请求 ${book.requested_currency} / 实际 ${depth.currency} · 抓取于 ${new Date(book.observed_at*1000).toLocaleString()} · 快照 ${book.id.slice(0,8)}`,
    actions,
  );
  const kpis=renderKpiGrid([
    {label:'最低卖价 (Ask 1)',value:s.best_ask?native(s.best_ask.minor,depth.currency):'无卖单',tone:'good'},
    {label:'最高买价 (Bid 1)',value:s.best_bid?native(s.best_bid.minor,depth.currency):'无买单'},
    {label:'买卖毛价差',value:s.spread_minor===null?'未知':native(s.spread_minor,depth.currency)},
    {label:'可见挂单总量 (卖/买)',value:`${s.visible_ask_quantity} / ${s.visible_bid_quantity}`,hint:'仅前十档可见深度'},
  ]);

  const depthSplit=rNode('div','depth-split-grid');
  depthSplit.append(
    renderDepthCard('Steam 卖盘前 10 档','ask',depth.asks,p=>native(p,depth.currency)),
    renderDepthCard('Steam 买盘前 10 档','bid',depth.bids,p=>native(p,depth.currency)),
  );

  box.replaceChildren(banner,kpis,depthSplit);

  if(book.warnings?.length) {
    const det=node('details');
    det.append(node('summary',`边界提示 (${book.warnings.length} 条)`));
    const warnings=node('ul');
    for(const warning of book.warnings) {
      warnings.append(node('li',warning==='平台实际返回非 CNY；保留原币，不换汇，不能用于人民币预算'
        ?'平台实际返回非 CNY；保留原币，当前显示与预算通过独立有效汇率快照换算。':warning));
    }
    det.append(warnings);
    box.append(det);
  }
}

function renderListingPreview(preview) {
  const box=el('listing-preview-result'),a=preview.analysis;
  const reasons={empty_asks:'无卖盘',single_level:'分析范围只有一档',regular_undercut:'常规压价',gap_jump:'断层跳档'};
  const kpis=renderKpiGrid([
    {label:`原币挂牌 (${a.book_currency})`,value:a.native_gross_listing_estimate?native(a.native_gross_listing_estimate.minor,a.book_currency):'未知',tone:'good'},
    {label:`折合 ${preview.display_currency}`,value:preview.display_gross_listing_estimate?native(preview.display_gross_listing_estimate.minor,preview.display_currency):'未知'},
    {label:'净钱包估算',value:preview.display_net_estimate?native(preview.display_net_estimate.minor,preview.display_currency):'未知'},
  ]);
  const note=node('p',`判定：${reasons[a.reason]||a.reason} · 剔除最低 ${a.ignored_lowest_tiers.length} 档 · ${a.stopped_by_blocking_volume?'阻挡量超限停档':'未触达阻挡上限'} · ${a.floor_applied?'已保护下限':'未触发下限'}`);
  note.className='hint';
  box.replaceChildren(kpis,note);
  if(a.wall) {
    const wallP=node('p',`墙分析截至卖盘第 ${a.wall.level_index+1} 档 (${native(a.wall.price_minor,a.book_currency)})，剔除后累计 ${a.wall.cumulative_quantity} 件。`);
    wallP.className='hint';
    box.append(wallP);
  }
  box.append(pricingTrace(a));
}

el('listing-fees-enabled').onchange=event=>{el('listing-fees').disabled=!event.target.checked;};
el('listing-preview-form').onsubmit=async event=>{
  event.preventDefault();if(!selectedBookId||pricingBusy)return;
  const id=selectedBookId,display_currency=el('book-display-currency').value,data=new FormData(event.target),integer=name=>Number(data.get(name));
  const policy=priceFields.read();
  const fee_schedule=data.has('estimate_fees')?feeFields.read():null;
  pricingBusy=true;el('listing-preview-create').disabled=true;el('listing-preview-feedback').textContent='正在使用本地快照计算…';
  try {
    const preview=await parallax.listingPricePreview(id,{policy,display_currency,max_quote_age_secs:integer('max_quote_age_secs'),fee_schedule});
    if(selectedBookId!==id||el('book-display-currency').value!==display_currency)return;
    renderListingPreview(preview);el('listing-preview-feedback').textContent='本地研究计算完成；未创建订单、上架或修改预算。';
  } catch(e){if(selectedBookId===id)el('listing-preview-feedback').textContent=`计算失败：${e.message}；输入与上次结果保留`;}
  finally {pricingBusy=false;el('listing-preview-create').disabled=!selectedBookId;}
};

async function history() {
  const {items}=await parallax.orderBooks();
  const buttons=items.map(book=>{
    const button=node('button',undefined);
    button.type='button';button.className='history-row';
    const left=node('strong',book.request.market_hash_name);
    const right=node('span',`${book.book.currency} · ${new Date(book.observed_at*1000).toLocaleDateString()}`);
    button.append(left,right);
    button.onclick=async()=>{
      try {render(await parallax.orderBook(book.id));el('book-feedback').textContent='已读取历史快照，未刷新平台报价。';}
      catch(e){el('book-feedback').textContent=e.message;}
    };return button;
  });
  el('book-history').replaceChildren(...(buttons.length?buttons:[rNode('p','hint','尚无盘口快照；点击上方「读取并保存」即可即时抓取。')]));
  if(!selectedBookId&&items.length){
    try{render(await parallax.orderBook(items[0].id));}catch(_){}
  }
}

el('order-book-form').onsubmit=async event=>{
  event.preventDefault();el('book-fetch').disabled=true;el('book-feedback').textContent='正在读取公开盘口…';
  try {
    const data=new FormData(event.target);
    const book=await parallax.fetchOrderBook({app_id:730,market_hash_name:data.get('market_hash_name'),currency:data.get('currency')});
    render(book);el('book-feedback').textContent='盘口快照已保存；请查看实际币种与买卖深度。';
    try {await history();} catch(e){el('book-feedback').textContent=`快照已保存，历史刷新失败：${e.message}`;}
  } catch(e){el('book-feedback').textContent=`读取失败：${e.message}；输入与上次结果保留。`;}
  finally {el('book-fetch').disabled=false;}
};

document.querySelectorAll('.book-preset-btn').forEach(btn=>{
  btn.onclick=()=>{
    const input=el('order-book-form')?.elements?.market_hash_name;
    if(input){
      input.value=btn.dataset.name||'';
      el('order-book-form').requestSubmit();
    }
  };
});

history().catch(e=>el('book-feedback').textContent=`盘口历史读取失败：${e.message}`);
