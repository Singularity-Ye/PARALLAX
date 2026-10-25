import {parallax} from './lib/api.js';
import {pricingEditor,feeEditor} from './lib/pricing-fields.js';
import {exitValuationView} from './lib/pricing-view.js';
const el=id=>document.getElementById(`strategy-${id}`);
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const option=(value,text)=>{const n=node('option',text);n.value=value;return n;};
const date=t=>new Date(t*1000).toLocaleString();
const labels={pass:'满足研究规则',fail:'未满足',unknown:'无法判断'};
const modules={quote_freshness:'报价有效期',fx_freshness:'汇率有效期',strict_ratio:'钱包折算率',position_capacity:'仓位额度',history_coverage:'历史覆盖',sample_cv:'样本 CV',trend7:'七日趋势',calendar_p10:'七日收益 P10',stress_exit_ratio:'压力净卖出折算率'};
const exitPricing=pricingEditor(el('exit-pricing'));
const exitFees=feeEditor(el('exit-fees'),{chooseCurrency:true});
let loaded=null,plans=[],busy=false,refreshing=false,choicesRequest=0,versionRequest=0;
const number=id=>Number(el(id).value);
function fields(){
  const holding=el('objective').value==='holding';
  el('ratio').disabled=busy||holding;
  if(holding)el('history').checked=true;
  el('history').disabled=busy||holding;
  el('objective-help').textContent=holding?'可在独立持有研究中选择保存历史，无需榜单或预算；满足规则不代表低估或未来收益。':'钱包方案比较现金成本与估算Steam净钱包；钱包金额不等于可提现现金。';
  el('history-fields').disabled=!el('history').checked;
  el('exit-enabled').disabled=busy||holding||!el('history').checked;
  el('exit-fields').disabled=holding||!el('history').checked||!el('exit-enabled').checked;
  for(const name of ['cv','trend','p10'])el(`${name}-fields`).disabled=!el(`${name}-enabled`).checked;
}
for(const id of ['objective','history','cv-enabled','trend-enabled','p10-enabled','exit-enabled'])el(id).onchange=fields;
function ready(){el('holding-evaluate').disabled=busy||refreshing||!loaded||loaded.draft.objective!=='holding'||!el('account').value||!el('holding-choices').querySelector('input:checked');el('evaluate').disabled=busy||refreshing||!loaded||!el('plan').value;el('save').disabled=busy||refreshing||(!loaded&&!!el('select').value);}
function showVersion(v){
  loaded=v;const d=v.draft;
  el('objective').value=d.objective??'wallet_conversion';
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
  const objective=el('objective').value,holding=objective==='holding';
  const rules=holding?[]:[{module:'strict_ratio',max_ratio_bps:number('ratio')}];
  if(el('position').checked)rules.push({module:'position_capacity'});
  if(el('history').checked){
    rules.push({module:'history_coverage',min_usable_days:number('min-days'),max_missing_days:number('missing-days'),max_age_secs:number('history-age'),max_reference_lag_days:number('lag')});
    if(el('cv-enabled').checked)rules.push({module:'sample_cv',basis:el('cv-basis').value,max_cv_bps:number('cv')});
    if(el('trend-enabled').checked)rules.push({module:'trend7',min_points:number('trend-points'),min_slope_bps_per_day:number('slope-min'),max_slope_bps_per_day:number('slope-max'),min_r_squared_bps:number('r2')});
    if(el('p10-enabled').checked)rules.push({module:'calendar_p10',min_pairs:number('pairs'),min_return_bps:number('p10'),slippage_bps:number('slippage')});
  }
  if(!holding&&el('history').checked&&el('exit-enabled').checked)rules.push({module:'stress_exit_ratio',max_ratio_bps:number('exit-ratio'),parameters:{pricing:exitPricing.read(),fees:exitFees.read(),min_pairs:number('exit-pairs'),slippage_bps:number('exit-slippage'),max_book_age_secs:number('exit-age')}});
  return {schema_version:3,objective,name:el('name').value.trim(),description:el('description').value,rules};
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
  busy=value;for(const id of ['form','evaluation-form','holding-form'])for(const control of el(id).querySelectorAll('button,input,select'))control.disabled=value;
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
  const [histories,books]=await Promise.all([id?parallax.priceHistories(id):Promise.resolve({items:[]}),plan?parallax.orderBooks():Promise.resolve({items:[]})]);
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
  if(ticket===choicesRequest){holdingChoices(histories.items);el('history-choices').replaceChildren(fragment);el('book-choices').replaceChildren(bookFragment);ready();await holdingRecords(id);}
}
for(const id of ['account','plan'])el(id).onchange=()=>{
  if(id==='account'){holdingChoices([]);el('holding-result').replaceChildren();el('holding-records').replaceChildren();el('holding-feedback').textContent='';}
  choicesRequest++;el('history-choices').replaceChildren(node('p','正在读取可引用的历史…'));el('book-choices').replaceChildren();
  choices().catch(e=>{el('evaluation-feedback').textContent=`历史选择读取失败：${e.message}`;});ready();
};
function render(e){
  const holding=e.strategy.draft.objective==='holding';
  const outcome=e.status==='pass'&&holding?'满足持有观察规则（非买入建议）':labels[e.status];
  const box=el('result');box.replaceChildren(node('h3',`${e.strategy.draft.name} · v${e.strategy.revision} · ${outcome}`),node('p',`${date(e.created_at)} · ${holding?'持有观察':'换Steam余额'} · ${e.plan.source==='demo'?'演示数据':'SteamDT 候选'} · ${e.items.length} 件计划商品 · 仅研究，未下单`));
  if(!e.items.length)box.append(node('p','计划中没有可研究的商品，不能作为通过。'));
  for(const item of e.items){
    const section=node('details');section.open=true;section.append(node('summary',`${item.item_key} · ${labels[item.status]}`));
    if(item.review){
      const review=item.review,basis={demo:'演示数据',steamdt_reference:'SteamDT参考价（非实时可成交卖单）',buff_visible_sell_snapshot:'历史BUFF可见卖单快照（非锁定报价）'};
      section.append(node('p',`现金价格依据：${basis[review.cash_price_basis]??review.cash_price_basis} · 证据时间 ${date(review.price_evidence_at)}`));
      if(review.unchecked_rules.length)section.append(node('p',`未启用的规则：${review.unchecked_rules.map(k=>modules[k]??k).join('、')}；不属于本次已检查范围。`));
      const gaps=node('details');gaps.append(node('summary','未评估事项与人工复核'));
      for(const text of review.unassessed)gaps.append(node('p',text));
      for(const text of review.manual_checks)gaps.append(node('p',`人工确认：${text}`));
      section.append(gaps);
    }else section.append(node('p','旧研究未保存目标/价格依据与未评估清单，不在回看时补算。'));
    renderRuleDecisions(section,item);
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
    const e=await parallax.evaluateStrategy(request);render(e);window.dispatchEvent(new Event('parallax:research-saved'));el('evaluation-feedback').textContent='研究预览已保存，未发起平台请求或创建订单。';
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

function holdingChoices(items){
  const checked=new Set([...el('holding-choices').querySelectorAll('input:checked')].map(n=>n.value));
  const fragment=document.createDocumentFragment();
  for(const h of items){const label=node('label'),input=node('input');input.type='checkbox';input.value=h.id;input.checked=checked.has(h.id);input.onchange=ready;
    label.append(input,node('span',`${h.request.item_key} · ${h.series.currency??'币种未知'} · ${date(h.observed_at)} · ${h.request.manual_item?'指定名称':h.request.inventory_snapshot_id?'库存来源':'扫描来源'}`));fragment.append(label);}
  if(!items.length)fragment.append(node('p','当前账户暂无历史。请到价格历史页指定市场名称读取，或从库存商品进入。选择账户后刷新可用证据。'));
  el('holding-choices').replaceChildren(fragment);
}
let currentHolding=null;
function renderHolding(report){
  currentHolding=report;
  const box=el('holding-result');box.replaceChildren(node('h3',`${report.strategy.draft.name} · ${labels[report.status]} · 仅历史观察`),node('p',`${date(report.created_at)} · ${report.items.length} 个显式选择商品 · 不依赖榜单或预算`));
  if(report.unchecked_rules.length)box.append(node('p',`未启用规则：${report.unchecked_rules.map(k=>modules[k]??k).join('、')}`));
  for(const item of report.items){const section=node('details');section.open=true;section.append(node('summary',`${item.item_key} · ${labels[item.status]}`));renderRuleDecisions(section,item);
    const add=node('button','关注此商品');add.type='button';add.onclick=()=>{el('watch-name').value=item.item_key;el('watch-reason').focus();};section.append(add);box.append(section);}
  for(const text of report.limitations)box.append(node('p',text,'hint'));
}
async function holdingRecords(account){
  if(!account){el('holding-records').replaceChildren();return;}
  const {items}=await parallax.holdingResearches(account);if(account!==el('account').value)return;
  const buttons=items.map(r=>{const b=node('button',`${r.strategy_name} · ${labels[r.status]} · ${date(r.created_at)}`);b.type='button';b.className='history-row';b.onclick=async()=>{try{const report=await parallax.holdingResearch(account,r.id);if(el('account').value!==account)return;renderHolding(report);el('holding-feedback').textContent='已载入保存时的独立研究，未重新采集或计算。';}catch(e){el('holding-feedback').textContent=e.message;}};return b;});
  el('holding-records').replaceChildren(...buttons);
}
el('holding-form').onsubmit=async event=>{
  event.preventDefault();if(busy||!loaded)return;const account=el('account').value;
  const request={strategy_id:loaded.strategy_id,revision:loaded.revision,history_snapshot_ids:[...el('holding-choices').querySelectorAll('input:checked')].map(n=>n.value)};
  lock(true);el('holding-feedback').textContent='正在研究明确选择的保存历史，不请求平台…';
  try{const report=await parallax.createHoldingResearch(account,request);renderHolding(report);el('holding-feedback').textContent='独立研究已保存，未创建预算或订单。';try{await holdingRecords(account);}catch(e){el('holding-feedback').textContent=`研究已保存，列表刷新失败：${e.message}`;}}
  catch(e){el('holding-feedback').textContent=`未保存新研究：${e.message}；选择与原结果保留。`;}
  finally{lock(false);}
};
function renderRuleDecisions(section,item){
  for(const rule of item.decisions){
    section.append(node('p',`${modules[rule.module]??rule.module}：${labels[rule.status]}。${rule.reason}`));
    if(rule.observed!==null){const details=node('details');details.append(node('summary','查看本项证据'),node('pre',JSON.stringify(rule.observed,null,2)));section.append(details);}
  }
}

let watchingBusy=false;
const watchAccount=()=>el('account').value;
async function showObservations(account,item){
 const {items}=await parallax.watchObservations(account,item.id);if(watchAccount()!==account)return;
 const box=el('watch-observations');box.replaceChildren(node('h3',`${item.item_key} · 按登记顺序的观察记录`));
 if(!items.length)box.append(node('p','尚无观察；打开包含此商品的独立研究后追加。'));
 for(const o of items){const r=o.research,i=r.items.find(i=>i.item_key===item.item_key),h=r.histories.find(h=>h.request.item_key===item.item_key);
 const section=node('details');section.append(node('summary',`${date(o.recorded_at)} · ${labels[i.status]}`),node('p',`策略 ${r.strategy.draft.name} v${r.strategy.revision} · 研究 ${date(r.created_at)} · 历史采集 ${date(h.observed_at)} · ${h.series.currency??'币种未知'}`));
 renderRuleDecisions(section,i);section.append(node('p',`未启用：${r.unchecked_rules.map(k=>modules[k]??k).join('、')||'无'}；历史观察不代表收益或现金价值。`));box.append(section);
 }
}
async function refreshWatches(){
 const account=watchAccount();if(!account){el('watch-list').replaceChildren();el('watch-observations').replaceChildren();return;}
 const {items}=await parallax.watchlist(account);if(watchAccount()!==account)return;
 const box=el('watch-list');box.replaceChildren();
 for(const item of items){const row=node('div');row.append(node('h4',item.item_key),node('p',item.reason));
 const view=node('button','查看观察记录');view.type='button';view.onclick=()=>showObservations(account,item).catch(e=>el('watch-feedback').textContent=e.message);
 const save=node('button','追加当前打开的研究');save.type='button';save.onclick=async()=>{
 if(watchingBusy)return;
 if(watchAccount()!==account||currentHolding?.account_id!==account||!currentHolding.items.some(i=>i.item_key===item.item_key)){el('watch-feedback').textContent='请先打开本账户包含此商品的独立研究。';return;}
 watchingBusy=true;save.disabled=true;
 try{await parallax.observeWatch(account,item.id,{research_id:currentHolding.id});el('watch-feedback').textContent='观察已保存（相同研究不重复登记），未采集或重算。';await showObservations(account,item);}
 catch(e){el('watch-feedback').textContent=`本次操作未完成：${e.message}；可重试。`;}
 finally{watchingBusy=false;save.disabled=false;}
 };const history=node('button','选择并读取新历史');history.type='button';history.onclick=()=>window.dispatchEvent(new CustomEvent('parallax:watch-history',{detail:{account_id:account,item_key:item.item_key}}));row.append(view,save,history);box.append(row);}
 if(!items.length)box.append(node('p','当前账户暂无关注商品。'));
}
el('watch-refresh').onclick=()=>refreshWatches().catch(e=>el('watch-feedback').textContent=e.message);
el('account').addEventListener('change',()=>{currentHolding=null;el('watch-list').replaceChildren();el('watch-observations').replaceChildren();el('watch-feedback').textContent='';refreshWatches().catch(e=>el('watch-feedback').textContent=e.message);});
el('watch-form').onsubmit=async event=>{event.preventDefault();if(watchingBusy)return;const account=watchAccount();if(!account){el('watch-feedback').textContent='请先选择研究账户。';return;}watchingBusy=true;
 try{await parallax.addWatch(account,{item_key:el('watch-name').value,reason:el('watch-reason').value});el('watch-feedback').textContent='关注已保存；已有同名关注会保留首次原因。';await refreshWatches();}
 catch(e){el('watch-feedback').textContent=`关注未保存：${e.message}；输入保留。`;}
 finally{watchingBusy=false;}
};

window.addEventListener('parallax:watch-item',async({detail})=>{
 try{await refresh();if(![...el('account').options].some(o=>o.value===detail.account_id))throw new Error('账户不存在');
 el('account').value=detail.account_id;el('account').dispatchEvent(new Event('change'));el('watch-name').value=detail.item_key;el('watch-reason').value='';location.hash='strategies';el('watch-reason').focus();el('watch-feedback').textContent='已带入商品和账户，填写原因后添加关注。';}
 catch(e){el('watch-feedback').textContent=e.message;}
});
