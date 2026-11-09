import { parallax } from './lib/api.js';
import { readWalletPolicy, readPositionInputs, renderPositionInputs } from './plan-inputs.js';
import {readSteamSnapshotIds} from './orderbook.js';
import {renderBuffCandidates,readBuffQuoteInputs} from './buff-quotes.js';
import {renderHistoryCandidates} from './price-history.js';
import {renderTicker} from './lib/ticker.js';
const el = id => document.getElementById(id);
const money = value => `¥${(value / 100).toFixed(2)}`;
const reasonNames = {page_budget:'已达到页数预算，仍可能有未读取商品',item_budget:'已达到分析预算',source_exhausted:'当前来源查询已返回空页（不代表全市场）',source_error:'来源请求失败，保留已取得结果',cancelled:'已按要求停止',process_restart:'上次扫描被进程重启中断'};
const statusNames = {running:'扫描中',completed:'已完成',failed:'失败',cancelled:'已停止',interrupted:'已中断'};
let current = null, activeId = null, polling = false;
function feedback(message, error = false) { el('feedback').textContent = message; el('feedback').classList.toggle('error',error); }
function node(tag, text, className) { const n = document.createElement(tag); if(text !== undefined) n.textContent = text; if(className) n.className = className; return n; }
function render(report) {
  current = report;
  window.dispatchEvent(new CustomEvent('parallax:scan-selected',{detail:report}));
  el('plan-create').disabled = report.status !== 'completed';
  el('plan-source').textContent = `基于当前扫描 ${report.id.slice(0,8)} · ${report.request.source === 'demo' ? '演示' : 'SteamDT'} · ${statusNames[report.status] || report.status}`;
  el('source-label').textContent = report.request.source === 'demo' ? '固定演示数据 · 非真实行情' : 'SteamDT / BUFF 榜单';
  feedback(report.status === 'running' ? '正在扫描来源，结果将在本轮结束后显示。可停止扫描。' : `${reasonNames[report.stop_reason] || report.stop_reason}。${report.error || ''}`, report.status === 'failed');
  const c = report.coverage;
  el('metrics').replaceChildren(...[['读取页数',c.pages_received],['收到条目',c.rows_received],['已分析',c.examined],['预筛通过',c.accepted]].map(([label,value]) => {
    const box = node('div',undefined,'metric');
    const valBox = node('b');
    renderTicker(valBox, String(value));
    box.append(valBox,node('span',label));
    return box;
  }));
  if(el('result-summary')) el('result-summary').textContent = `淘汰 ${c.rejected} · 重复 ${c.duplicates} · 已收未分析 ${c.unexamined_received}`;
  renderCandidates();
  renderPositionInputs(report);
  renderBuffCandidates(report);
  renderHistoryCandidates(report);
}
function renderCandidates() {
  if (!current) return;
  const rows = current.candidates.filter(c => !el('only-passed').checked || c.passed);
  const tbody = el('candidates-tbody');
  if (tbody) {
    if (!rows.length) {
      tbody.innerHTML = '<tr class="empty-row"><td colspan="8" class="empty-cell">当前没有符合筛选条件的商品；可调整上方过滤范围。</td></tr>';
    } else {
      const fragment = document.createDocumentFragment();
      for (const c of rows) {
        const d = c.deep_analysis || {};
        const tr = document.createElement('tr');
        tr.className = c.passed ? 'candidate-row pass' : 'candidate-row fail';

        // 1. Name & key
        const tdName = document.createElement('td');
        tdName.className = 'col-name';
        const strong = document.createElement('strong');
        strong.textContent = c.item.name;
        tdName.appendChild(strong);
        const subKey = document.createElement('div');
        subKey.style.cssText = 'font-size:11px;color:var(--text-muted,#94a3b8);margin-top:2px;';
        subKey.textContent = c.item.key;
        tdName.appendChild(subKey);
        tr.appendChild(tdName);

        // 2. BUFF Cash
        const tdCash = document.createElement('td');
        tdCash.className = 'col-mono';
        tdCash.textContent = c.item.cash_cents > 0 ? money(c.item.cash_cents) : '—';
        tr.appendChild(tdCash);

        // 3. Steam Smart Barrier / Net Wallet
        const tdSteam = document.createElement('td');
        tdSteam.className = 'col-mono';
        const netStr = c.estimated_wallet_cents > 0 ? money(c.estimated_wallet_cents) : '—';
        const grossStr = c.item.steam_gross_cents > 0 ? money(c.item.steam_gross_cents) : '—';
        tdSteam.innerHTML = `<div>${netStr} <span style="font-size:10px;color:#38bdf8;">净得</span></div><div style="font-size:11px;color:var(--text-muted,#94a3b8);">壁垒 ${grossStr}</div>`;
        tr.appendChild(tdSteam);

        // 4. Ratio
        const tdRatio = document.createElement('td');
        tdRatio.className = 'col-mono';
        const ratioStr = c.ratio_bps === null ? '—' : `${(c.ratio_bps / 100).toFixed(2)}%`;
        const ratioTag = document.createElement('span');
        ratioTag.className = c.ratio_bps !== null && c.ratio_bps <= 7500 ? 'ratio-pill good' : 'ratio-pill';
        ratioTag.textContent = ratioStr;
        tdRatio.appendChild(ratioTag);
        tr.appendChild(tdRatio);

        // 5. Volume & Sell Pressure
        const tdVol = document.createElement('td');
        tdVol.className = 'col-mono';
        const sp = d.sell_pressure !== undefined ? d.sell_pressure.toFixed(2) : '—';
        const spColor = d.sell_pressure_ok !== false ? '#22c55e' : '#f59e0b';
        tdVol.innerHTML = `<div>${c.item.volume ?? '—'} <span style="font-size:10px;color:var(--text-muted);">件/日</span></div><div style="font-size:11px;color:${spColor};">卖压比 ${sp}</div>`;
        tr.appendChild(tdVol);

        // 6. 30D Stability & Trend
        const tdStab = document.createElement('td');
        tdStab.className = 'col-mono';
        const trendLabel = d.trend_status === 'RISING' ? '↗ 上升' : (d.trend_status === 'FALLING' ? '↘ 回落' : '→ 横盘');
        const trendColor = d.trend_status === 'RISING' ? '#22c55e' : '#38bdf8';
        const cvStr = d.cv_30d !== undefined ? d.cv_30d.toFixed(3) : '—';
        const pctStr = d.price_percentile !== undefined ? `P${d.price_percentile}` : '—';
        tdStab.innerHTML = `<div style="color:${trendColor};font-weight:600;font-size:12px;">${trendLabel} · ${pctStr}</div><div style="font-size:11px;color:var(--text-muted,#94a3b8);">CV ${cvStr} · R² ${d.r_squared !== undefined ? d.r_squared.toFixed(2) : '—'}</div>`;
        tr.appendChild(tdStab);

        // 7. Holdings & Safe Purchase Limit
        const tdLimit = document.createElement('td');
        tdLimit.className = 'col-mono';
        const held = d.held_same_units ?? 0;
        const safeLim = d.safe_purchase_limit ?? '—';
        tdLimit.innerHTML = `<div>限购 <strong style="color:#38bdf8;">${safeLim}</strong> 件</div><div style="font-size:11px;color:${held > 0 ? '#f59e0b' : 'var(--text-muted,#94a3b8)'};">已持仓 ${held} 件</div>`;
        tr.appendChild(tdLimit);

        // 8. Passed badge & Quick Action
        const tdAction = document.createElement('td');
        tdAction.className = 'col-action';
        const badge = document.createElement('span');
        badge.className = c.passed ? 'status-pill status-pill-success' : 'status-pill status-pill-active';
        badge.textContent = c.passed ? '✓ 深度通过' : '未通过';
        badge.title = c.rules.map(r => `${r.passed ? '✓' : '×'} ${r.rule}: ${r.reason}`).join('\n') + (d.summary ? `\n${d.summary}` : '');
        tdAction.appendChild(badge);
        const planLink = document.createElement('a');
        planLink.href = '#planning';
        planLink.className = 'table-action-link';
        planLink.style.marginLeft = '6px';
        planLink.textContent = '配平 ↗';
        tdAction.appendChild(planLink);
        tr.appendChild(tdAction);

        fragment.appendChild(tr);
      }
      tbody.replaceChildren(fragment);
    }
  }
}
function busy(value) { el('start').disabled=value;el('cancel').hidden=!value;el('cancel').disabled=false; }
async function history() {
  const data = await parallax.scans();
  el('history-list').replaceChildren(...data.items.map(report => {
    const button = node('button',undefined,'history-row');button.type='button';
    button.append(node('strong',`${report.request.source==='demo'?'演示':'SteamDT'} · ${statusNames[report.status] || report.status} · 通过 ${report.coverage.accepted} 件`),node('span',new Date(report.created_at*1000).toLocaleString()));
    button.onclick=async()=>{try{render(await parallax.scan(report.id));el('records-feedback').textContent='';location.hash='workspace';}catch(e){el('records-feedback').textContent=`扫描记录读取失败：${e.message}；列表保留。`;}};return button;
  }));
  if(!data.items.length) el('history-list').append(node('p','还没有扫描记录。','hint'));
  return data.items;
}
el('scan-form').addEventListener('submit',async event=>{
  event.preventDefault();
  const f=new FormData(event.target), n=key=>Number(f.get(key));
  const request={source:f.get('source'),max_pages:n('max_pages'),page_size:n('page_size'),max_items:n('max_items'),min_price_cents:Math.round(n('min_price')*100),max_price_cents:Math.round(n('max_price')*100),min_volume:n('min_volume'),max_ratio_bps:Math.round(n('max_ratio')*100),exclude_keywords:String(f.get('exclude_keywords')).split(/[,，]/).map(s=>s.trim()).filter(Boolean)};
  request.wallet_policy=readWalletPolicy(f);
  busy(true);
  try { const report=await parallax.startScan(request);activeId=report.id;render(report);await history(); }
  catch(e){feedback(e.message,true);busy(Boolean(activeId));}
});
el('cancel').onclick=async()=>{if(!activeId)return;el('cancel').disabled=true;try{const result=await parallax.cancelScan(activeId);feedback(result.message);}catch(e){feedback(e.message,true);el('cancel').disabled=false;}};
el('only-passed').onchange=renderCandidates;
setInterval(async()=>{
  if(!activeId||polling)return;polling=true;
  try{const report=await parallax.scan(activeId);render(report);if(report.status!=='running'){activeId=null;busy(false);await history();}}
  catch(e){feedback(`状态读取失败：${e.message}；记录仍保留，稍后重试。`,true);}finally{polling=false;}
},1000);
(async()=>{try{const items=await history();const running=items.find(r=>r.status==='running');if(running){activeId=running.id;busy(true);}if(items.length)render(await parallax.scan(running?.id || items[0].id));}catch(e){el('history-list').textContent=`记录读取失败：${e.message}`;}})();

function planFeedback(message, failed = false) {
  el('plan-feedback').textContent = message;
  el('plan-feedback').classList.toggle('error', failed);
}
window.addEventListener('parallax:quote-referenced',({detail})=>planFeedback(`已添加 ${detail.source} 报价引用；生成计划时将重新校验商品、账户及双方时效。`));
function renderPlan(plan) {
  window.dispatchEvent(new CustomEvent('parallax:plan-selected',{detail:plan}));
  const box = el('plan-result');
  const source = plan.source === 'demo' ? '固定演示数据' : `${plan.buff_valuations?.length?'BUFF 首屏现金报价':'SteamDT 现金榜单'} + ${plan.steam_valuations?.length?'Steam 盘口估值':'SteamDT 钱包估值'}`;
  const posSourceLabel = plan.position_source === 'auto_inventory_and_ledger'
    ? '仓位来源：已自动关联当前账户真实 Steam 库存与在途采购台账'
    : (plan.position_source==='manual_unverified'?'仓位来源：用户手动填报，未平台核验':'未使用账户仓位快照');
  box.replaceChildren(node('h3', `${source} · 预算预览`),
    node('p', `扫描 ${plan.request.scan_id.slice(0,8)} · 生成于 ${new Date(plan.created_at*1000).toLocaleString()}`),
    node('p', `计划现金 ${money(plan.cash_total_cents)} · 已填报预留 ${money(plan.reserved_cash_cents || 0)} · 剩余预算 ${money(plan.remaining_budget_cents)} · 估算钱包 ${money(plan.estimated_wallet_total_cents)}`),
    node('p', `${plan.wallet_policy?.kind==='parameterized_fees'?'自填费用模型估算':'1.15 比例估算'} · ${posSourceLabel}`,'hint'));
  const policyLimits=plan.request.liquidity_policy;
  if(plan.research_origin)box.append(node('p',`重新规划来源：研究 ${plan.research_origin.evaluation_id} · 原预算 ${plan.research_origin.parent_plan_id} · 策略v${plan.research_origin.strategy.revision}。旧结论仅作选品依据，需对新计划重新研究。`,'hint'));
  if(policyLimits)box.append(node('p',`本计划限购：日量约束${policyLimits.volume_limit_enabled!==false?'启用':'未启用'}；低价保护${policyLimits.low_price_guard_enabled!==false?'启用':'未启用'}。硬上限及所引用持有/未决扣减仍保留。`,'hint'));
  if (plan.target_met !== null) box.append(node('p', plan.target_met ? '已达到估算钱包目标（整件分配可能略超目标）。' : '未达到估算钱包目标，请查看预算与跳过原因。'));
  for(const evidence of plan.steam_valuations || [])box.append(node('p',`Steam 估值引用 ${evidence.snapshot_id} · ${evidence.market_hash_name} · 最低卖价 ${money(evidence.gross_listing_estimate_cents)}；为慢卖估算，不保证成交。`,'hint'));
  if(plan.fx)box.append(node('p',`外币按参考汇率 ${plan.fx.id} 折为人民币，报价日期 ${[...new Set(Object.values(plan.fx.rates).map(r=>r.date))].join(' / ')}；原币及完整汇率已随计划保存。`,'hint'));
  for(const evidence of plan.buff_valuations || [])box.append(node('p',`BUFF 引用 ${evidence.snapshot_id} · ${evidence.item_key} · ${evidence.unit_cash_cents===null?'空卖盘':money(evidence.unit_cash_cents)} · 最低价可见 ${evidence.lowest_tier_visible_units} 条；未锁定数量。`,'hint'));
  for (const line of plan.lines) {
    const row = node('article', undefined, 'candidate');
    const ratioPct = line.candidate.ratio_bps ? `${(line.candidate.ratio_bps / 100).toFixed(2)}%` : '—';
    const d = line.candidate.deep_analysis || {};
    row.append(
      node('strong', `${line.candidate.item.name}  × ${line.quantity} 件`),
      node('p', `单价 ${money(line.candidate.item.cash_cents)} · 现金合计 ${money(line.cash_total_cents)} · 预计 Steam 税后净得 ${money(line.estimated_wallet_total_cents)} · 折算比 ${ratioPct}`),
      node('p', `安全限购上限 ${line.max_additional_units ?? d.safe_purchase_limit ?? '—'} 件（当前已持仓 ${d.held_same_units ?? 0} 件 · 24h 日量 ${line.candidate.item.volume} 件）`, 'hint')
    );
    if (line.candidate.valuation) row.append(node('p',`每件估算费用 ${money(line.candidate.valuation.fees_minor)} · 无法用于净值的价差余量 ${money(line.candidate.valuation.unallocated_gross_minor)}`,'hint'));
    box.append(row);
  }
  if (!plan.lines.length) {
    box.append(node('p', '本计划未分配商品；请检查报价有效期、预算与预筛条件。'));
  } else {
    const commitBar = node('div');
    commitBar.style.cssText = 'margin:14px 0;padding:12px 14px;border-radius:8px;background:rgba(34,197,94,0.1);border:1px solid rgba(34,197,94,0.3);display:flex;align-items:center;justify-content:space-between;gap:12px;';
    const tip = node('span', `共规划 ${plan.lines.length} 个品类（合计 ${money(plan.cash_total_cents)}）。采购完成后可一键批量写入「持有饰品」台账并自动追踪收货与挂牌：`);
    tip.style.cssText = 'font-size:12px;color:var(--text-secondary);';
    const commitBtn = node('button', '📦 一键按此计划建仓入账 →');
    commitBtn.type = 'button';
    commitBtn.className = 'btn btn-sm btn-primary';
    commitBtn.style.cssText = 'background:#16a34a;border-color:#22c55e;white-space:nowrap;';
    commitBtn.onclick = async () => {
      commitBtn.disabled = true;
      try {
        const accRes = await parallax.accounts();
        const targetAcc = plan.request.position_snapshot?.account_id || accRes.items?.[0]?.id;
        if (!targetAcc) throw new Error('请先在「账号管理」中添加一个账户');
        const nowSec = Math.floor(Date.now() / 1000);
        for (const line of plan.lines) {
          await parallax.createPurchaseLot(targetAcc, {
            client_key: crypto.randomUUID(),
            item_key: line.candidate.item.key,
            quantity: line.quantity,
            cash_cost_cents: line.cash_total_cents,
            purchased_at: nowSec,
            purchase_ref: `plan_${plan.id.slice(0, 8)}_${line.candidate.item.goods_id || 'item'}`
          });
        }
        window.dispatchEvent(new CustomEvent('parallax:ledger-changed', {detail: {account: targetAcc}}));
        planFeedback(`✓ 已将本计划 ${plan.lines.length} 个采购批次一键写入「持有饰品」台账！`);
        location.hash = 'purchases';
      } catch (e) {
        planFeedback(`建仓入账失败：${e.message}`, true);
        commitBtn.disabled = false;
      }
    };
    commitBar.append(tip, commitBtn);
    box.append(commitBar);
  }
  const detail = node('details');detail.append(node('summary',`跳过 ${plan.skipped.length} 件商品 · 查看原因`));
  const reasons = node('ul'); for (const item of plan.skipped) reasons.append(node('li',`${item.key}：${item.reason}`));
  detail.append(reasons);box.append(detail);
  const limits = node('ul', undefined, 'hint');for (const blocker of plan.execution_blockers) limits.append(node('li', blocker));
  box.append(limits);
}
async function planHistory() {
  const {items} = await parallax.plans();
  el('plan-history').replaceChildren(...items.map(plan => {
    const button = node('button', `${plan.source === 'demo' ? '演示' : 'SteamDT'} · ${money(plan.cash_total_cents)} · ${new Date(plan.created_at*1000).toLocaleString()}`, 'history-row');
    button.type='button';
    button.onclick = async () => {try {renderPlan(await parallax.plan(plan.id));el('records-feedback').textContent='';planFeedback('已读取保存的历史预览；它不是当前可执行报价。');location.hash='planning';} catch(e) {el('records-feedback').textContent=`预算记录读取失败：${e.message}；列表保留。`;}};
    return button;
  }));
  if (!items.length) el('plan-history').append(node('p','还没有保存的预算计划。','hint'));
}
el('plan-form').addEventListener('submit', async event => {
  event.preventDefault();
  if (!current || current.status !== 'completed') {planFeedback('请先完成一次扫描。',true);return;}
  const form = new FormData(event.target);
  const target = String(form.get('target_wallet')).trim();
  const request = {scan_id:current.id,budget_cents:Math.round(Number(form.get('budget'))*100),
    target_wallet_cents:target ? Math.round(Number(target)*100) : null,
    max_units_per_item:Number(form.get('max_units')),max_positions:Number(form.get('max_positions')),
    max_quote_age_secs:Number(form.get('quote_age'))};
  el('plan-create').disabled=true;
  try {
    Object.assign(request,readPositionInputs());
    request.steam_snapshot_ids=readSteamSnapshotIds();
    Object.assign(request,readBuffQuoteInputs());
    renderPlan(await parallax.createPlan(request));
    planFeedback('预算预览已保存。未创建订单或预留资金。');
    try {await planHistory();} catch(e) {planFeedback(`计划已保存，但历史列表刷新失败：${e.message}`,true);}
  } catch(e) {planFeedback(e.message,true);}
  finally {el('plan-create').disabled=current?.status !== 'completed';}
});
planHistory().catch(e=>el('plan-history').textContent=`计划记录读取失败：${e.message}；请检查本地服务后重试。`);
window.addEventListener('parallax:plan-replanned',({detail})=>{renderPlan(detail);location.hash='planning';planFeedback('已打开按研究选择的新预算；请对新数量重新保存研究。');planHistory().catch(e=>planFeedback(`预算已保存，列表刷新失败：${e.message}`,true));});

async function runAutoPlan(budgetYuan, maxUnits = 50, maxPositions = 8) {
  const budget_cents = Math.max(100, Math.round(Number(budgetYuan || 200) * 100));
  const payload = {
    scan_id: current?.status === 'completed' ? current.id : null,
    budget_cents,
    max_units_per_item: Number(maxUnits || 50),
    max_positions: Number(maxPositions || 8),
  };
  try {
    planFeedback('正在结合当前账户真实库存、在途台账与 5% 日量安全限购上限自动配平...');
    const plan = await parallax.createAutoPlan(payload);
    renderPlan(plan);
    planFeedback(`⚡ 智能建仓配平完成！共分配 ${plan.lines.length} 种商品，合计消耗 ¥${(plan.cash_total_cents / 100).toFixed(2)}，预计 Steam 税后到账 ¥${(plan.estimated_wallet_total_cents / 100).toFixed(2)}。`);
    await planHistory();
    location.hash = 'planning';
  } catch (e) {
    planFeedback(`智能配平失败：${e.message}`, true);
    feedback(`智能配平失败：${e.message}`, true);
  }
}

if (el('workspace-auto-plan-btn')) {
  el('workspace-auto-plan-btn').onclick = () => {
    const b = el('workspace-auto-budget')?.value || 200;
    if (el('auto-plan-budget')) el('auto-plan-budget').value = b;
    runAutoPlan(b, 50, 8);
  };
}
if (el('auto-plan-submit')) {
  el('auto-plan-submit').onclick = () => {
    const b = el('auto-plan-budget')?.value || 200;
    const u = el('auto-plan-max-units')?.value || 50;
    const p = el('auto-plan-max-pos')?.value || 8;
    runAutoPlan(b, u, p);
  };
}
