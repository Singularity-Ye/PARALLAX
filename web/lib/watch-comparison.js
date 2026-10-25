import {parallax} from './api.js';
import {node,nativeMoney} from './pricing-view.js';
const date=t=>new Date(t*1000).toLocaleString();
const day=d=>d==null?'未知':new Date(d*86400000).toISOString().slice(0,10);
const labels={pass:'满足',fail:'未满足',unknown:'未知'};
const format=v=>v==null?'未知':`${(v/100).toFixed(2)}%`;
const names={raw_cv_bps:'原日价CV',clean_cv_bps:'清洗后CV',trend_bps_per_day:'七日相对斜率/日',calendar_p10_bps:'七日历史P10'};
function renderComparison(box,c,renderRules){
 box.replaceChildren(node('h4',`${c.item_key} · 固定规则对照`),node('p',`固定版本：${c.fixed_strategy.draft.name} v${c.fixed_strategy.revision}；后续原研究版本：${c.later_original_strategy.draft.name} v${c.later_original_strategy.revision}`),node('p',`基准冻结 ${date(c.baseline_fixed_at)}；采集 ${date(c.baseline_collected_at)} → ${date(c.later_collected_at)}`),node('p',`冻结后开始采集：${c.collection_after_baseline?'是':'否，仅回顾'}；新增完整历史日：${c.new_completed_day?'是':'否'}；相同统计口径：${c.same_measurement_basis?'是':'否'}`));
 const price=p=>`${day(p.reference_utc_day)} · ${p.price?nativeMoney(p.price.minor,p.price.currency):'价格未知'}`;
 box.append(node('p',`参考日价：${price(c.before_price)} → ${price(c.after_price)}`),node('p',`历史日价变化：${c.historical_price_change_bps==null?'不计算（缺少新完整日、同口径价格或币种证据）':format(c.historical_price_change_bps)}；非扣费收益。`));
 const table=node('table'),head=node('tr');for(const title of ['指标','基准','后续','变化（百分点）'])head.append(node('th',title));table.append(head);
 for(const m of c.metrics){const row=node('tr');for(const text of [names[m.key]??m.key,format(m.before),format(m.after),m.delta==null?'不计算':(m.delta/100).toFixed(2)])row.append(node('td',text));table.append(row);}box.append(table);
 const rules=node('section');rules.append(node('h4',`判断：${labels[c.before.status]} → ${labels[c.after_fixed_rules.status]}（沿用基准规则）`));
 for(const [title,item]of [['基准原判断',c.before],['后续按固定规则判断',c.after_fixed_rules]]){const d=node('details');d.open=true;d.append(node('summary',title));renderRules(d,item);rules.append(d);}box.append(rules);
 const unchanged=node('p',`未启用规则与原始参数保留于固定版本；后续重算时点 ${date(c.later_research_at)}，没有用今天时间改写历史。`);box.append(unchanged);
 for(const text of c.limitations)box.append(node('p',text,'hint'));
}
export async function comparisonPanel(account,item,observations,isCurrent,renderRules){
 const section=node('section');section.append(node('h4','固定基准与观察对照'));
 const hint=node('p','选择基准后会固定此关注商品的观察与策略版本，不提供覆盖；只有后续明确采集的新历史才能对照。');section.append(hint);
 let baseline=(await parallax.watchBaseline(account,item.id)).item;
 const feedback=node('p');feedback.setAttribute('role','status');const result=node('div');
 const select=node('select');select.setAttribute('aria-label','基准观察');
 const later=node('select');later.setAttribute('aria-label','后续观察');
 const option=o=>{const n=node('option',`${date(o.recorded_at)} · ${labels[o.research.items.find(i=>i.item_key===item.item_key).status]} · ${o.id.slice(0,8)}`);n.value=o.id;return n;};
 const fix=node('button','固定所选基准');fix.type='button';const compare=node('button','对照所选后续观察');compare.type='button';let busy=false;
 function ready(){select.disabled=busy||!!baseline||!observations.length;fix.disabled=busy||!!baseline||!observations.length;later.disabled=busy||!baseline||!later.options.length;compare.disabled=later.disabled;
 hint.textContent=baseline?`已固定基准 ${date(baseline.observation.recorded_at)}，冻结于 ${date(baseline.fixed_at)}；策略 ${baseline.observation.research.strategy.draft.name} v${baseline.observation.research.strategy.revision}。不会以新版本覆盖。`:'选择基准后固定此商品观察与策略版本，不能覆盖；原未通过/未知结果也能作为基准。';}
 function choices(){select.replaceChildren(...observations.map(option));if(baseline)select.value=baseline.observation.id;
 later.replaceChildren(...observations.filter(o=>o.id!==baseline?.observation.id).map(option));ready();if(baseline&&!later.options.length)feedback.textContent='等待新的观察：读取后续历史并保存独立研究，再追加到此关注商品。当前不会拿同一记录自我对照。';}
 fix.onclick=async()=>{if(busy||!isCurrent())return;busy=true;ready();try{baseline=await parallax.fixWatchBaseline(account,item.id,{observation_id:select.value});if(!isCurrent())return;feedback.textContent='基准已固定并持久化；等待后续新历史。';choices();}catch(e){if(isCurrent())feedback.textContent=`固定未完成：${e.message}；选择保留。`;}finally{busy=false;ready();}};
 compare.onclick=async()=>{if(busy||!isCurrent())return;busy=true;ready();try{const c=await parallax.compareWatch(account,item.id,{observation_id:later.value});if(!isCurrent())return;renderComparison(result,c,renderRules);feedback.textContent='对照完成：仅使用保存证据，不请求平台、不修改原研究。';}catch(e){if(isCurrent())feedback.textContent=`对照未完成：${e.message}；选择和原结果保留。`;}finally{busy=false;ready();}};
 section.append(select,fix,later,compare,feedback,result);choices();return section;
}
