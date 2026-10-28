// Dashboard presentation only. Prices, fees and allocation amounts come from saved backend evidence.
import {parallax} from './api.js';
import {node} from './pricing-view.js';
const el=id=>document.getElementById(id);
const amount=value=>Number.isSafeInteger(value)&&value>=0?`CNY ${(value/100).toFixed(2)}`:'未知';
const ratio=value=>Number.isFinite(value)?`${(value/100).toFixed(2)}%`:'未知';
let scan=null, planRevision=0;
function showFriction(){
  const c=scan?.candidates?.find(c=>c.item.key===el('friction-item').value);
  el('ratio-marker').hidden=true;
  if(!c)return;
  el('friction-cash').textContent=c.item.cash_cents>0?amount(c.item.cash_cents):'未知';
  el('friction-net').textContent=c.valuation?.net?.currency==='CNY'?amount(c.valuation.net.minor):'未知';
  el('friction-fee').textContent=c.valuation?.net?.currency==='CNY'?amount(c.valuation.fees_minor):'未知';
  el('friction-ratio').textContent=ratio(c.ratio_bps);
  if(Number.isFinite(c.ratio_bps)){
    el('ratio-marker').hidden=false;el('ratio-marker').style.left=`${Math.min(100,Math.max(0,c.ratio_bps/200))}%`;
    el('ratio-track').setAttribute('aria-label',`估算折算率${ratio(c.ratio_bps)}，标尺0至200%；超界仅截断图形`);
  }else el('ratio-track').setAttribute('aria-label','该商品折算率未知');
  el('friction-scope').textContent=`${scan.request.source==='demo'?'演示样例':'SteamDT参考榜单'} · ${new Date(c.item.observed_at*1000).toLocaleString()} · ${scan.request.wallet_policy?.kind==='parameterized_fees'?'自填费用模型':'1.15比例模型'}；标尺0–200%，非实时盘口/滑点/收益率。来源成交量统计窗口未单独确认。`;
}
export function renderCockpitScan(report){
  scan=report;
  const rows=report?.candidates??[];
  el('dashboard-empty-candidates').hidden=rows.length>0;
  el('dashboard-candidates-table').hidden=rows.length===0;
  el('dashboard-candidates-table').style.display=rows.length?'table':'none';
  el('dashboard-empty-candidates').style.display=rows.length?'none':'flex';
  el('dashboard-pass-rate').textContent=`预筛 ${report?.coverage?.accepted??0} / ${report?.coverage?.examined??0}；不是成交命中率`;
  const fragment=document.createDocumentFragment();
  for(const c of rows.slice(0,8)){
    const row=node('tr');
    for(const text of [c.item.name,c.item.steam_gross_cents>0?amount(c.item.steam_gross_cents):'未知',c.item.cash_cents>0?amount(c.item.cash_cents):'未知',ratio(c.ratio_bps),String(c.item.volume),c.passed?'预筛通过':'未通过'])row.append(node('td',text));
    const action=node('td');const link=node('a','查看预算');link.href='#planning';link.className='text-link';action.append(link);row.append(action);fragment.append(row);
  }
  el('dashboard-candidates-tbody').replaceChildren(fragment);
  const previous=el('friction-item').value;
  el('friction-item').replaceChildren();
  for(const c of rows){const option=node('option',c.item.name);option.value=c.item.key;el('friction-item').append(option);}
  el('friction-item').disabled=!rows.length;
  if(rows.some(c=>c.item.key===previous))el('friction-item').value=previous;
  if(rows.length)showFriction();else{
    for(const id of ['cash','net','fee','ratio'])el(`friction-${id}`).textContent='未知';
    el('ratio-marker').hidden=true;el('ratio-track').setAttribute('aria-label','没有候选折算率证据');
    el('friction-scope').textContent='本次扫描没有可展示候选；不代表全市场无商品。';
  }
}
function renderPlan(plan){
  const budget=plan.request.budget_cents,planned=plan.cash_total_cents,reserved=plan.reserved_cash_cents??0,remaining=plan.remaining_budget_cents;
  const valid=[budget,planned,reserved,remaining].every(v=>Number.isSafeInteger(v)&&v>=0)&&budget>0&&planned+reserved+remaining===budget;
  el('dashboard-budget-val').textContent=amount(budget);
  el('dashboard-wallet-val').textContent=amount(plan.estimated_wallet_total_cents);
  el('dashboard-reserved-val').textContent=`填报预留 ${amount(reserved)}`;
  // Update high-density telemetry allocation bar
  const totalCapEl = el('telemetry-total-capital');
  const barAvail = el('alloc-bar-avail');
  const barResv = el('alloc-bar-resv');
  const barHold = el('alloc-bar-hold');
  const legAvail = el('legend-avail-val');
  const legResv = el('legend-resv-val');
  const legHold = el('legend-hold-val');
  if (totalCapEl) totalCapEl.textContent = `总额度: ${amount(budget)}`;
  if (legAvail) legAvail.textContent = amount(remaining);
  if (legResv) legResv.textContent = amount(reserved);
  if (legHold) legHold.textContent = amount(planned);
  if (barAvail && valid) barAvail.style.width = `${Math.max(0, remaining/budget*100)}%`;
  if (barResv && valid) barResv.style.width = `${Math.max(0, reserved/budget*100)}%`;
  if (barHold && valid) barHold.style.width = `${Math.max(0, planned/budget*100)}%`;
  el('dashboard-plan-scope').textContent=`预算 ${plan.id.slice(0,8)} · ${new Date(plan.created_at*1000).toLocaleString()} · ${plan.source==='demo'?'演示':'参考报价'} · 历史预览，非余额/锁单。`;
  el('capital-planned').style.width=valid?`${planned/budget*100}%`:'0%';
  el('capital-reserved').style.width=valid?`${reserved/budget*100}%`:'0%';
  el('capital-track').setAttribute('aria-label',valid?`预算${amount(budget)}，计划${amount(planned)}，填报预留${amount(reserved)}，剩余${amount(remaining)}`:'预算金额无法组成完整比例，未显示水位');
  el('capital-planned-value').textContent=amount(planned);el('capital-reserved-value').textContent=amount(reserved);el('capital-remaining-value').textContent=amount(remaining);
  const position=plan.request.position_snapshot;
  const positionText=position?`手工填报 · 账户${position.account_id} · ${new Date(position.observed_at*1000).toLocaleString()}；不是实时持仓`:'未引用账户仓位；不是零持仓';
  el('capital-position').textContent=`${positionText}。单品硬上限${plan.request.max_units_per_item}件，种类上限${plan.request.max_positions}。`;
  el('sentinel-position').textContent=positionText;
  const policy=plan.request.liquidity_policy;
  el('sentinel-risk').textContent=policy?`本计划：日量约束${policy.volume_limit_enabled!==false?'启用':'未启用'}，低价保护${policy.low_price_guard_enabled!==false?'启用':'未启用'}`:'本计划未显式引用限购策略；不能推断已启用';
}
export function initCockpit(){
  el('friction-item').onchange=showFriction;
  window.addEventListener('parallax:plan-selected',({detail})=>{++planRevision;renderPlan(detail);});
  const revision=planRevision;
  parallax.plans().then(async({items})=>{
    if(!items.length)return;
    const latest=[...items].sort((a,b)=>b.created_at-a.created_at)[0];const saved=await parallax.plan(latest.id);
    if(planRevision===revision)renderPlan(saved);
  }).catch(e=>{if(planRevision===revision)el('dashboard-plan-scope').textContent=`预算读取失败：${e.message}；未显示资金水位。`;});
  parallax.proxyStatus().then(value=>{el('sentinel-route').textContent=value.mode==='explicit_routes'?'按平台固定路由；非自动轮换':'沿用系统环境；不等于直连';})
    .catch(e=>{el('sentinel-route').textContent=`配置未读取：${e.message}`;});
}
