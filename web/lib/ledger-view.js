import {node,nativeMoney} from './pricing-view.js';
const money = value => value ? nativeMoney(value.minor,value.currency) : '未知';
function metricCell(metric,format=money) {
  const td=node('td',metric?.value == null ? '未知' : format(metric.value));
  td.className='ledger-number';
  td.title=metric?.reason || '人工登记，未平台核验';
  return td;
}
export function renderLedgerTable(body,page) {
  body.replaceChildren();
  if (!page?.items.length) {
    const tr=node('tr'),td=node('td',page?'本页没有台账记录。':'尚未读取台账。');
    td.colSpan=13;td.className='empty-table-td';tr.append(td);body.append(tr);return;
  }
  for (const row of page.items) {
    const tr=node('tr'),checkCell=node('td'),check=node('input');
    check.type='checkbox';check.className='holding-item-cb';check.value=row.row_id;check.setAttribute('aria-label',`选择 ${row.item_key}`);checkCell.append(check);
    const item=node('td');item.append(node('strong',row.item_key),node('small',`${row.quantity} 件 · 人工登记未核验`));
    const cost=node('td',money(row.purchase_cost));cost.title='整行数量的总成本，不是单件价';cost.className='ledger-number';
    const state=node('td',({pending_receipt:'待收货',held:'持有',listed:'已登记挂牌',sold:'已登记成交'})[row.stage]);
    const action=node('td'),button=node('button','管理');button.type='button';button.className='btn-action-lot secondary';button.dataset.lotId=row.lot_id;action.append(button);
    tr.append(checkCell,node('td',new Date(row.purchased_at*1000).toLocaleString()),item,node('td',row.asset_id??'未绑定'),cost,
      metricCell(row.purchase_market_price),metricCell(row.current_market_price),metricCell(row.sale_net_wallet),
      metricCell(row.actual_discount_bps,v=>`${(v/100).toFixed(2)}%`),metricCell(row.cash_profit),state,metricCell(row.market_change),action);
    body.append(tr);
  }
}
export function renderLedgerSummary(get,summary) {
  const values={
    'summary-held-units':summary?`${summary.counts.held_units} 件（已挂 ${summary.counts.listed_units} / 待收 ${summary.counts.pending_units} / 已售 ${summary.counts.sold_units}）`:'未知',
    'summary-total-cost':summary?money(summary.open_purchase_cost):'未知',
    'summary-total-value':'未知',
    'summary-total-profit':summary?.total_cash_profit.value==null?'未知':money(summary.total_cash_profit.value),
    'summary-avg-ratio':summary?.actual_discount_bps.value==null?'未知':`${(summary.actual_discount_bps.value/100).toFixed(2)}%`
  };
  for (const [id,text] of Object.entries(values)) { const target=get(id);target.textContent=text;target.className='summary-val'; }
  get('summary-total-value').title='尚无完整持仓现价证据';
  get('summary-total-profit').title=summary?.total_cash_profit.reason??'尚未读取';
  get('summary-avg-ratio').title=summary?.actual_discount_bps.reason??'尚未读取';
}
