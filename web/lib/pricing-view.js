export const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export const nativeMoney=(minor,currency)=>new Intl.NumberFormat('zh-CN',{style:'currency',currency}).format(minor/100);
export function pricingTrace(a){
  const trace=node('details');trace.append(node('summary','剔除与断层依据'));const rows=node('ul');
  for(const t of a.ignored_lowest_tiers)rows.append(node('li',`剔除 ${nativeMoney(t.price_minor,a.book_currency)} · ${t.quantity} 件`));
  for(const g of a.gap_checks)rows.append(node('li',`${nativeMoney(g.lower_minor,a.book_currency)} → ${nativeMoney(g.upper_minor,a.book_currency)} · 阻挡量 ${g.blocking_quantity} · 阈值取 ${nativeMoney(g.absolute_threshold_minor,'CNY')} 与 ${g.relative_threshold_bps/100}% 中较高者 · ${g.jumped?'严格超过，跳档':'未严格超过，不跳档'}`));
  if(!rows.children.length)rows.append(node('li','没有剔除或可检查的相邻断层。'));trace.append(rows);return trace;
}
export function exitValuationView(v){
  const section=node('section'),money=m=>nativeMoney(m.minor,m.currency),q=v.pricing.native_gross_listing_estimate;
  section.append(node('h4','出售估值 · 原币费用与压力场景'),node('p',`当前挂牌参考 ${q?money(q):'未知'} · 压力毛价 ${money(v.stressed_gross_native)}`),
    node('p',`普通净钱包折CNY ${money(v.normal_net_cny)} · 压力净钱包折CNY ${money(v.stressed_net_cny)}`),
    node('p',`历史七日配对 ${v.sample_count} 个 · P10 ${(v.return_p10_bps/100).toFixed(2)}% · 一次压力滑点 ${(v.slippage_bps/100).toFixed(2)}%`));
  section.append(node('p',v.stressed_fees?`压力原币净额 ${money(v.stressed_fees.net)} · 模型费用 ${nativeMoney(v.stressed_fees.fees_minor,v.stressed_fees.net.currency)}；不是实际结算或未来成交保证。`:'按所选费用模型没有可负担的正净额，不能通过压力净折算率。'));
  section.append(pricingTrace(v.pricing));return section;
}
