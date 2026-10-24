import {parallax} from './lib/api.js';
import {pricingEditor,feeEditor} from './lib/pricing-fields.js';
import {exitValuationView} from './lib/pricing-view.js';
const el=id=>document.getElementById(`strategy-${id}`);
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const option=(value,text)=>{const n=node('option',text);n.value=value;return n;};
const date=t=>new Date(t*1000).toLocaleString();
const labels={pass:'满足研究规则',fail:'未满足',unknown:'无法判断'};
const modules={quote_freshness:'报价有效期',fx_freshness:'汇率有效期',strict_ratio:'严格折算率',position_capacity:'仓位额度',history_coverage:'历史覆盖',sample_cv:'样本 CV',trend7:'七日趋势',calendar_p10:'七日收益 P10',stress_exit_ratio:'压力净卖出折算率'};
const exitPricing=pricingEditor(el('exit-pricing'));
const exitFees=feeEditor(el('exit-fees'),{chooseCurrency:true});
let loaded=null,plans=[],busy=false,refreshing=false,choicesRequest=0,versionRequest=0;
const number=id=>Number(el(id).value);
function fields(){
  el('history-fields').disabled=!el('history').checked;
  el('exit-enabled').disabled=busy||!el('history').checked;
  el('exit-fields').disabled=!el('history').checked||!el('exit-enabled').checked;
  for(const name of ['cv','trend','p10'])el(`${name}-fields`).disabled=!el(`${name}-enabled`).checked;
}
for(const id of ['history','cv-enabled','trend-enabled','p10-enabled','exit-enabled'])el(id).onchange=fields;
function ready(){el('evaluate').disabled=busy||refreshing||!loaded||!el('plan').value;el('save').disabled=busy||refreshing||(!loaded&&!!el('select').value);}
function showVersion(v){
  loaded=v;const d=v.draft;
  el('name').value=d.name;el('description').value=d.description;
  for(const key of ['position','history','cv-enabled','trend-enabled','p10-enabled','exit-enabled'])el(key).checked=false;
  for(const rule of d.rules){
    switch(rule.module){
      case 'stress_exit_ratio':el('exit-enabled').checked=true;el('exit-ratio').value=rule.max_ratio_bps;el('exit-age').value=rule.parameters.max_book_age_secs;el('exit-pairs').value=rule.parameters.min_pairs;el('exit-slippage').value=rule.parameters.slippage_bps;exitPricing.set(rule.parameters.pricing);exitFees.set(rule.parameters.fees);break;
      case 'strict_ratio':el('ratio').value=rule.max_ratio_bps;break;
      case 'position_capacity':el('position').checked=true;break;
      case 'history_coverage':el('history').checked=true;el('min-days').value=rule.min_usable_days;el('missing-days').value=rule.max_missing_days;el('history-age').value=rule.max_age_secs;el('lag').value=rule.max_reference_lag_days;break;
      case 'sample_cv':el('cv-enabled').checked=true;el('cv-basis').value=rule.basis;el('cv').value=rule.max_cv_bps;break;
      case 'trend7':el('trend-enabled').checked=true;el('trend-points').value=rule.min_points;el('slope-min').value=rule.min_slope_bps_per_day;el('slope-max').value=rule.max_slope_bps_per_day;el('r2').value=rule.min_r_squared_bps;break;
      case 'calendar_p10':el('p10-enabled').checked=true;el('pairs').value=rule.min_pairs;el('p10').value=rule.min_return_bps;el('slippage').value=rule.slippage_bps;break;
    }
  }
  el('version-info').textContent=`研究使用已保存版本 v${v.revision} · ${date(v.created_at)}。编辑后需保存；旧版继续保留。`;
  fields();ready();
}
function draft(){
  const rules=[{module:'strict_ratio',max_ratio_bps:number('ratio')}];
  if(el('position').checked)rules.push({module:'position_capacity'});
  if(el('history').checked){
    rules.push({module:'history_coverage',min_usable_days:number('min-days'),max_missing_days:number('missing-days'),max_age_secs:number('history-age'),max_reference_lag_days:number('lag')});
    if(el('cv-enabled').checked)rules.push({module:'sample_cv',basis:el('cv-basis').value,max_cv_bps:number('cv')});
    if(el('trend-enabled').checked)rules.push({module:'trend7',min_points:number('trend-points'),min_slope_bps_per_day:number('slope-min'),max_slope_bps_per_day:number('slope-max'),min_r_squared_bps:number('r2')});
    if(el('p10-enabled').checked)rules.push({module:'calendar_p10',min_pairs:number('pairs'),min_return_bps:number('p10'),slippage_bps:number('slippage')});
  }
  if(el('history').checked&&el('exit-enabled').checked)rules.push({module:'stress_exit_ratio',max_ratio_bps:number('exit-ratio'),parameters:{pricing:exitPricing.read(),fees:exitFees.read(),min_pairs:number('exit-pairs'),slippage_bps:number('exit-slippage'),max_book_age_secs:number('exit-age')}});
  return {schema_version:rules.some(r=>r.module==='stress_exit_ratio')?2:1,name:el('name').value.trim(),description:el('description').value,rules};
}
async function versions(id,revision){
  const ticket=++versionRequest;
  const {items}=await parallax.strategyVersions(id);
  if(ticket!==versionRequest||el('select').value!==id)return;
  el('version').replaceChildren(...items.map(v=>option(v.revision,`v${v.revision} · ${v.draft.name} · ${date(v.created_at)}`)));
  el('version').disabled=busy||!items.length;
  const selected=items.find(v=>v.revision===revision)??items[0];
  if(selected){el('version').value=String(selected.revision);showVersion(selected);}
}
el('select').onchange=async()=>{
  loaded=null;ready();
  if(!el('select').value){el('new').click();return;}
  try{await versions(el('select').value);}catch(e){el('save-feedback').textContent=e.message;}
};
el('version').onchange=async()=>{
  const id=el('select').value,rev=number('version'),ticket=++versionRequest;loaded=null;ready();
  try{const v=await parallax.strategyVersion(id,rev);if(ticket===versionRequest&&el('select').value===id&&number('version')===rev)showVersion(v);}catch(e){el('save-feedback').textContent=e.message;}
};
el('new').onclick=()=>{
  versionRequest++;loaded=null;el('select').value='';el('version').replaceChildren();el('version').disabled=true;
  el('form').reset();fields();ready();el('save-feedback').textContent='';el('version-info').textContent='新策略尚未保存。以下为研究参数示例，效果需要另行验证。';
};
async function heads(){
  const {items}=await parallax.strategies(),previous=el('select').value;
  el('select').replaceChildren(option('','新策略'),...items.map(v=>option(v.strategy_id,`${v.draft.name} · 最新 v${v.revision}`)));
  if(items.some(v=>v.strategy_id===previous))el('select').value=previous;
}
function lock(value){
  busy=value;for(const id of ['form','evaluation-form'])for(const control of el(id).querySelectorAll('button,input,select'))control.disabled=value;
  for(const id of ['new','select','version','refresh'])el(id).disabled=value;
  if(!value){fields();el('version').disabled=!loaded;ready();}
}
el('form').onsubmit=async event=>{
  event.preventDefault();if(busy)return;
  const value=draft(),previous=loaded;lock(true);el('save-feedback').textContent='正在保存不可覆盖的新版本…';
  try{
    const v=previous?await parallax.reviseStrategy(previous.strategy_id,{expected_revision:previous.revision,draft:value}):await parallax.createStrategy(value);
    showVersion(v);el('save-feedback').textContent=`已保存 v${v.revision}；没有激活自动交易。`;
    try{await heads();el('select').value=v.strategy_id;await versions(v.strategy_id,v.revision);}catch(e){el('save-feedback').textContent=`版本已保存，列表刷新失败：${e.message}`;}
  }catch(e){el('save-feedback').textContent=`本次未确认保存：${e.message}；输入保留。`;}
  finally{lock(false);}
};
async function choices(){
  const ticket=++choicesRequest,id=el('account').value,plan=plans.find(p=>p.id===el('plan').value);
  const previous=new Map([...el('history-choices').querySelectorAll('select')].map(s=>[s.dataset.historyItem,s.value]));
  const fragment=document.createDocumentFragment();
  const [histories,books]=await Promise.all([id&&plan?parallax.priceHistories(id):Promise.resolve({items:[]}),plan?parallax.orderBooks():Promise.resolve({items:[]})]);
  if(id&&plan){
    const {items}=histories;
    for(const line of plan.lines){
      const label=node('label',line.candidate.item.name),select=node('select');select.dataset.historyItem=line.candidate.item.key;
      select.append(option('','不引用历史（相关规则将无法判断）'));
      for(const h of items.filter(h=>h.request.scan_id===plan.request.scan_id&&h.request.item_key===line.candidate.item.key))select.append(option(h.id,`${date(h.observed_at)} · ${h.request.window_days} 日 · ${h.series.currency??'币种未知'} · ${h.id.slice(0,8)}`));
      if([...select.options].some(o=>o.value===previous.get(line.candidate.item.key)))select.value=previous.get(line.candidate.item.key);
      label.append(select);fragment.append(label);
    }
  }else fragment.append(node('p','选择账户和计划后，可逐件引用已有历史；没有历史也能研究，但相关规则保持未知。'));
  const bookFragment=document.createDocumentFragment(),previousBooks=new Map([...el('book-choices').querySelectorAll('select')].map(s=>[s.dataset.bookItem,s.value]));
  if(plan&&plan.source==='steamdt')for(const line of plan.lines){
    const label=node('label',`${line.candidate.item.name} · 出售盘口`),select=node('select');select.dataset.bookItem=line.candidate.item.key;select.append(option('','不引用盘口（压力净卖出规则将无法判断）'));
    for(const b of books.items.filter(b=>b.request.market_hash_name===line.candidate.item.key))select.append(option(b.id,`${date(b.observed_at)} · 实际 ${b.book.currency} · ${b.id.slice(0,8)}`));
    if([...select.options].some(o=>o.value===previousBooks.get(line.candidate.item.key)))select.value=previousBooks.get(line.candidate.item.key);label.append(select);bookFragment.append(label);
  }
  if(!bookFragment.childNodes.length)bookFragment.append(node('p','出售估值需真实计划中匹配商品的显式盘口；演示计划禁止引用平台证据。'));
  if(ticket===choicesRequest){el('history-choices').replaceChildren(fragment);el('book-choices').replaceChildren(bookFragment);ready();}
}
for(const id of ['account','plan'])el(id).onchange=()=>{
  choicesRequest++;el('history-choices').replaceChildren(node('p','正在读取可引用的历史…'));el('book-choices').replaceChildren();
  choices().catch(e=>{el('evaluation-feedback').textContent=`历史选择读取失败：${e.message}`;});ready();
};
function render(e){
  const box=el('result');box.replaceChildren(node('h3',`${e.strategy.draft.name} · v${e.strategy.revision} · ${labels[e.status]}`),node('p',`${date(e.created_at)} · ${e.plan.source==='demo'?'演示数据':'SteamDT 候选'} · ${e.items.length} 件计划商品 · 仅研究，未下单`));
  if(!e.items.length)box.append(node('p','计划中没有可研究的商品，不能作为通过。'));
  for(const item of e.items){
    const section=node('details');section.open=true;section.append(node('summary',`${item.item_key} · ${labels[item.status]}`));
    for(const rule of item.decisions){
      section.append(node('p',`${modules[rule.module]??rule.module}：${labels[rule.status]}。${rule.reason}`));
      if(rule.observed!==null){const details=node('details');details.append(node('summary','查看本项证据'),node('pre',JSON.stringify(rule.observed,null,2)));section.append(details);}
    }
    if(item.exit_valuation)section.append(exitValuationView(item.exit_valuation));
    box.append(section);
  }
  for(const text of e.execution_blockers)box.append(node('p',text));
}
async function records(){
  const {items}=await parallax.strategyEvaluations();const fragment=document.createDocumentFragment();
  for(const e of items){
    const b=node('button',`${e.strategy_name} · v${e.revision} · ${labels[e.status]} · ${date(e.created_at)}`);b.type='button';b.className='history-row';
    b.onclick=async()=>{try{render(await parallax.strategyEvaluation(e.id));el('evaluation-feedback').textContent='已载入当时的研究、规则和证据；未重新获取行情。';}catch(err){el('evaluation-feedback').textContent=err.message;}};fragment.append(b);
  }
  if(!items.length)fragment.append(node('p','还没有保存的研究预览。'));
  el('records').replaceChildren(fragment);
}
el('evaluation-form').onsubmit=async event=>{
  event.preventDefault();if(busy||!loaded)return;
  const request={strategy_id:loaded.strategy_id,revision:loaded.revision,plan_id:el('plan').value,account_id:el('account').value||null,history_snapshot_ids:[...el('history-choices').querySelectorAll('select')].map(s=>s.value).filter(Boolean),order_book_snapshot_ids:loaded.draft.rules.some(r=>r.module==='stress_exit_ratio')?[...el('book-choices').querySelectorAll('select')].map(s=>s.value).filter(Boolean):[]};
  lock(true);el('evaluation-feedback').textContent='正在按已保存版本研究证据…';
  try{
    const e=await parallax.evaluateStrategy(request);render(e);el('evaluation-feedback').textContent='研究预览已保存，未发起平台请求或创建订单。';
    try{await records();}catch(err){el('evaluation-feedback').textContent=`研究已保存，记录刷新失败：${err.message}`;}
  }catch(e){el('evaluation-feedback').textContent=`本次未保存新研究：${e.message}；输入与原结果保留。`;}
  finally{lock(false);}
};
async function refresh(){
  if(busy||refreshing)return;refreshing=true;el('refresh').disabled=true;ready();
  try{
    const results=await Promise.allSettled([heads(),records(),parallax.plans(),parallax.accounts()]);
    if(results[2].status==='fulfilled'){
      const previous=el('plan').value;plans=results[2].value.items;
      el('plan').replaceChildren(option('','选择已保存预算'),...plans.map(p=>option(p.id,`${date(p.created_at)} · ${p.source==='demo'?'演示':'SteamDT'} · ${p.lines.length} 件 · ${p.id.slice(0,8)}`)));
      if(plans.some(p=>p.id===previous))el('plan').value=previous;
    }
    if(results[3].status==='fulfilled'){
      const previous=el('account').value,accounts=results[3].value.items;
      el('account').replaceChildren(option('','未指定'),...accounts.map(a=>option(a.id,a.label)));
      if(accounts.some(a=>a.id===previous))el('account').value=previous;
    }
    const errors=results.filter(r=>r.status==='rejected').map(r=>r.reason.message);
    if(errors.length)el('evaluation-feedback').textContent=`部分列表未刷新：${errors.join('；')}`;
    try{await choices();}catch(e){el('evaluation-feedback').textContent=`历史选择未刷新：${e.message}`;}
    ready();
  }finally{refreshing=false;el('refresh').disabled=busy;ready();}
}
el('refresh').onclick=()=>refresh().catch(e=>el('evaluation-feedback').textContent=e.message);
window.addEventListener('parallax:pagechange',event=>{if(event.detail.id==='strategies')refresh().catch(e=>el('evaluation-feedback').textContent=e.message);});
fields();refresh().catch(e=>el('evaluation-feedback').textContent=e.message);
