import { parallax } from './lib/api.js';
import { readWalletPolicy, readPositionInputs, renderPositionInputs } from './plan-inputs.js';
import {readSteamSnapshotIds} from './orderbook.js';
import {renderBuffCandidates,readBuffQuoteInputs} from './buff-quotes.js';
import {renderHistoryCandidates} from './price-history.js';
const el = id => document.getElementById(id);
const money = value => `¥${(value / 100).toFixed(2)}`;
const reasonNames = {page_budget:'已达到页数预算，仍可能有未读取商品',item_budget:'已达到分析预算',source_exhausted:'当前来源查询已返回空页（不代表全市场）',source_error:'来源请求失败，保留已取得结果',cancelled:'已按要求停止',process_restart:'上次扫描被进程重启中断'};
const statusNames = {running:'扫描中',completed:'已完成',failed:'失败',cancelled:'已停止',interrupted:'已中断'};
let current = null, activeId = null, polling = false;
function feedback(message, error = false) { el('feedback').textContent = message; el('feedback').classList.toggle('error',error); }
function node(tag, text, className) { const n = document.createElement(tag); if(text !== undefined) n.textContent = text; if(className) n.className = className; return n; }
function render(report) {
  current = report;
  el('plan-create').disabled = report.status !== 'completed';
  el('plan-source').textContent = `基于当前扫描 ${report.id.slice(0,8)} · ${report.request.source === 'demo' ? '演示' : 'SteamDT'} · ${statusNames[report.status] || report.status}`;
  el('source-label').textContent = report.request.source === 'demo' ? '固定演示数据 · 非真实行情' : 'SteamDT / BUFF 榜单';
  feedback(report.status === 'running' ? '正在扫描来源，结果将在本轮结束后显示。可停止扫描。' : `${reasonNames[report.stop_reason] || report.stop_reason}。${report.error || ''}`, report.status === 'failed');
  const c = report.coverage;
  el('metrics').replaceChildren(...[['读取页数',c.pages_received],['收到条目',c.rows_received],['已分析',c.examined],['预筛通过',c.accepted]].map(([label,value]) => {
    const box = node('div',undefined,'metric');box.append(node('b',String(value)),node('span',label));return box;
  }));
  el('result-summary').textContent = `淘汰 ${c.rejected} · 重复 ${c.duplicates} · 已收未分析 ${c.unexamined_received}`;
  renderCandidates();
  renderPositionInputs(report);
  renderBuffCandidates(report);
  renderHistoryCandidates(report);
}
function renderCandidates() {
  if (!current) return;
  const rows = current.candidates.filter(c => !el('only-passed').checked || c.passed);
  el('candidates').replaceChildren(...rows.map(c => {
    const row = node('article',undefined,'candidate');
    const head = node('div',undefined,'candidate-head');head.append(node('h3',c.item.name),node('span',c.passed?'预筛通过':'未通过',`chip ${c.passed?'':'rejected'}`));
    const quotes = node('div',undefined,'quotes');
    for(const [label,value] of [['现金',money(c.item.cash_cents)],['估算钱包',money(c.estimated_wallet_cents)],['折算率',c.ratio_bps === null?'未知':`${(c.ratio_bps/100).toFixed(2)}%`]]) {
      const part = node('span',`${label} `);part.append(node('b',value));quotes.append(part);
    }
    const detail = node('details');detail.append(node('summary',`查看 ${c.rules.length} 条判断依据`));
    const list = node('ul');for(const rule of c.rules) list.append(node('li',`${rule.passed?'✓':'×'} ${rule.rule}：${rule.reason}`,rule.passed?'':'rule-fail'));
    detail.append(list);row.append(head,quotes,detail);return row;
  }));
  if(!rows.length) el('candidates').append(node('p','当前没有符合展示条件的商品；可调整范围或查看失败说明。','hint'));
}
function busy(value) { el('start').disabled=value;el('cancel').hidden=!value;el('cancel').disabled=false; }
async function history() {
  const data = await parallax.scans();
  el('history-list').replaceChildren(...data.items.map(report => {
    const button = node('button',undefined,'history-row');button.type='button';
    button.append(node('strong',`${report.request.source==='demo'?'演示':'SteamDT'} · ${statusNames[report.status] || report.status} · 通过 ${report.coverage.accepted} 件`),node('span',new Date(report.created_at*1000).toLocaleString()));
    button.onclick=async()=>{try{render(await parallax.scan(report.id));}catch(e){feedback(e.message,true);}};return button;
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
function renderPlan(plan) {
  const box = el('plan-result');
  const source = plan.source === 'demo' ? '固定演示数据' : `${plan.buff_valuations?.length?'BUFF 首屏现金报价':'SteamDT 现金榜单'} + ${plan.steam_valuations?.length?'Steam 盘口估值':'SteamDT 钱包估值'}`;
  box.replaceChildren(node('h3', `${source} · 预算预览`),
    node('p', `扫描 ${plan.request.scan_id.slice(0,8)} · 生成于 ${new Date(plan.created_at*1000).toLocaleString()}`),
    node('p', `计划现金 ${money(plan.cash_total_cents)} · 已填报预留 ${money(plan.reserved_cash_cents || 0)} · 剩余预算 ${money(plan.remaining_budget_cents)} · 估算钱包 ${money(plan.estimated_wallet_total_cents)}`),
    node('p', `${plan.wallet_policy?.kind==='parameterized_fees'?'自填费用模型估算':'1.15 比例估算'} · ${plan.position_source==='manual_unverified'?'仓位来源：用户手动填报，未平台核验':'未使用账户仓位快照'}`,'hint'));
  if (plan.target_met !== null) box.append(node('p', plan.target_met ? '已达到估算钱包目标（整件分配可能略超目标）。' : '未达到估算钱包目标，请查看预算与跳过原因。'));
  for(const evidence of plan.steam_valuations || [])box.append(node('p',`Steam 估值引用 ${evidence.snapshot_id} · ${evidence.market_hash_name} · 最低卖价 ${money(evidence.gross_listing_estimate_cents)}；为慢卖估算，不保证成交。`,'hint'));
  for(const evidence of plan.buff_valuations || [])box.append(node('p',`BUFF 引用 ${evidence.snapshot_id} · ${evidence.item_key} · ${evidence.unit_cash_cents===null?'空卖盘':money(evidence.unit_cash_cents)} · 最低价可见 ${evidence.lowest_tier_visible_units} 条；未锁定数量。`,'hint'));
  for (const line of plan.lines) {
    const row = node('article', undefined, 'candidate');
    row.append(node('strong', line.candidate.item.name),
      node('p', `${line.quantity} 件 · 现金合计 ${money(line.cash_total_cents)} · 估算钱包 ${money(line.estimated_wallet_total_cents)}`));
    if (line.candidate.valuation) row.append(node('p',`每件估算费用 ${money(line.candidate.valuation.fees_minor)} · 无法用于净值的价差余量 ${money(line.candidate.valuation.unallocated_gross_minor)}`,'hint'));
    box.append(row);
  }
  if (!plan.lines.length) box.append(node('p', '本计划未分配商品；请检查报价有效期、预算与预筛条件。'));
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
    button.onclick = async () => {try {renderPlan(await parallax.plan(plan.id));planFeedback('已读取保存的历史预览；它不是当前可执行报价。');} catch(e) {planFeedback(e.message,true);}};
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
planHistory().catch(e=>planFeedback(`计划记录读取失败：${e.message}`,true));
