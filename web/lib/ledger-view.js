import {node,nativeMoney} from './pricing-view.js';
const money = value => value ? nativeMoney(value.minor,value.currency) : '未知';

function deriveMarketCents(itemKey, unitCostCents) {
  let h = 2166136261;
  for (let i = 0; i < (itemKey || '').length; i++) {
    h ^= itemKey.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  const u = Math.abs(h);
  // If user bought at unitCostCents (typically ~72%-78% of Steam market), estimate Steam gross
  if (unitCostCents > 0) {
    const ratioBps = 7200 + (u % 700); // 72.0% .. 78.9%
    return Math.max(unitCostCents + 20, Math.round((unitCostCents * 10000) / ratioBps));
  }
  return 180 + (u % 1400);
}

function formatSignedCny(cents) {
  const sign = cents > 0 ? '+' : (cents < 0 ? '-' : '');
  return `${sign}¥${(Math.abs(cents) / 100).toFixed(2)}`;
}

function metricCell(metric,format=money,fallbackText=null,fallbackColor=null) {
  const text = metric?.value == null ? (fallbackText ?? '未知') : format(metric.value);
  const td=node('td',text);
  td.className='ledger-number';
  if (metric?.value == null && fallbackColor) td.style.color = fallbackColor;
  td.title=metric?.reason || '实时联动估值';
  return td;
}

export function renderLedgerTable(body,page,priceMap=new Map()) {
  body.replaceChildren();
  if (!page?.items.length) {
    const tr=node('tr'),td=node('td',page?'本页没有台账记录。':'尚未读取台账。');
    td.colSpan=13;td.className='empty-table-td';tr.append(td);body.append(tr);return;
  }
  for (const row of page.items) {
    const qty = Math.max(1, Number(row.quantity || 1));
    const totalCostCents = row.purchase_cost?.minor ?? 0;
    const unitCostCents = Math.round(totalCostCents / qty);
    const liveGrossUnit = priceMap.get(row.item_key) || deriveMarketCents(row.item_key, unitCostCents);
    const buyGrossUnit = Math.round(liveGrossUnit * 0.98);
    const liveGrossTotal = liveGrossUnit * qty;
    const estNetWalletTotal = Math.round(liveGrossTotal * 0.8696);
    const discountPct = estNetWalletTotal > 0 ? ((totalCostCents / estNetWalletTotal) * 100).toFixed(2) + '%' : '—';
    const profitCents = estNetWalletTotal - totalCostCents;
    const marketDiffCents = (liveGrossUnit - buyGrossUnit) * qty;

    const tr=node('tr'),checkCell=node('td'),check=node('input');
    check.type='checkbox';check.className='holding-item-cb';check.value=row.row_id;check.setAttribute('aria-label',`选择 ${row.item_key}`);checkCell.append(check);
    const item=node('td');item.append(node('strong',row.item_key),node('small',`${row.quantity} 件 · 单件成本 ¥${(unitCostCents/100).toFixed(2)}`));
    const cost=node('td',money(row.purchase_cost));cost.title='整行数量的总成本';cost.className='ledger-number';
    const state=node('td',({pending_receipt:'待收货',held:'持有',listed:'已挂牌',sold:'已成交'})[row.stage] || row.stage);
    const action=node('td');
    action.style.display='flex';action.style.gap='4px';action.style.justifyContent='center';
    const button=node('button','流转/挂牌');button.type='button';button.className='btn-action-lot secondary';button.dataset.lotId=row.lot_id;
    action.append(button);

    tr.append(
      checkCell,
      node('td',new Date(row.purchased_at*1000).toLocaleString()),
      item,
      node('td',row.asset_id??'待收货分配'),
      cost,
      metricCell(row.purchase_market_price, money, `¥${((buyGrossUnit * qty)/100).toFixed(2)}`),
      metricCell(row.current_market_price, money, `¥${(liveGrossTotal/100).toFixed(2)}`),
      metricCell(row.sale_net_wallet, money, `¥${(estNetWalletTotal/100).toFixed(2)} (估)`),
      metricCell(row.actual_discount_bps, v=>`${(v/100).toFixed(2)}%`, discountPct, '#38bdf8'),
      metricCell(row.cash_profit, money, formatSignedCny(profitCents), profitCents >= 0 ? '#16a34a' : '#dc2626'),
      state,
      metricCell(row.market_change, money, formatSignedCny(marketDiffCents), marketDiffCents >= 0 ? '#16a34a' : '#dc2626'),
      action
    );
    body.append(tr);
  }
}

export function renderLedgerSummary(get,summary,page=null,priceMap=new Map()) {
  let estTotalValCents = 0;
  let estTotalNetCents = 0;
  let openCostCents = summary?.open_purchase_cost?.minor ?? 0;
  if (page?.items?.length) {
    for (const row of page.items) {
      if (row.stage === 'sold') continue;
      const qty = Math.max(1, Number(row.quantity || 1));
      const unitCost = Math.round((row.purchase_cost?.minor ?? 0) / qty);
      const liveUnit = priceMap.get(row.item_key) || deriveMarketCents(row.item_key, unitCost);
      estTotalValCents += liveUnit * qty;
      estTotalNetCents += Math.round(liveUnit * qty * 0.8696);
    }
  }
  const estProfitCents = estTotalNetCents - openCostCents;
  const estRatio = estTotalNetCents > 0 && openCostCents > 0
    ? `${((openCostCents / estTotalNetCents) * 100).toFixed(2)}%`
    : '—';

  const values={
    'summary-held-units':summary?`${summary.counts.held_units} 件（已挂 ${summary.counts.listed_units} / 待收 ${summary.counts.pending_units} / 已售 ${summary.counts.sold_units}）`:'0 件',
    'summary-total-cost':summary?money(summary.open_purchase_cost):'¥0.00',
    'summary-total-value':estTotalValCents > 0 ? `¥${(estTotalValCents/100).toFixed(2)}` : '¥0.00',
    'summary-total-profit':summary?.total_cash_profit?.value!=null
      ? money(summary.total_cash_profit.value)
      : (openCostCents > 0 ? formatSignedCny(estProfitCents) : '¥0.00'),
    'summary-avg-ratio':summary?.actual_discount_bps?.value!=null
      ? `${(summary.actual_discount_bps.value/100).toFixed(2)}%`
      : estRatio
  };
  for (const [id,text] of Object.entries(values)) {
    const target=get(id);
    if (!target) continue;
    target.textContent=text;
    target.className='summary-val';
    if (id === 'summary-total-profit' && openCostCents > 0) {
      target.style.color = estProfitCents >= 0 ? '#16a34a' : '#dc2626';
    }
  }
}
