import {parallax} from './lib/api.js';
import {node,nativeMoney} from './lib/pricing-view.js';
import {decisionList,decisionLabels as label} from './lib/evidence-view.js';
const el=id=>document.getElementById(`preflight-${id}`);
const date=t=>new Date(t*1000).toLocaleString();
const names={source:'来源',selected_strategy:'所选策略重算',selected_quote_age:'报价时效',buff_binding:'BUFF 账户／价格／数量',steam_quote:'Steam 盘口',history_generation:'历史凭据代次',steam_session_metadata:'Steam 登录态元数据',buff_session_metadata:'BUFF 登录态元数据',buff_policy:'BUFF 访问限制',live_quote_refresh:'实时刷新',balance:'可用资金',position_ledger:'完整持仓',platform_fees:'平台实际费用',order_executor:'订单与对账'};
const option=(value,text)=>{const n=node('option',text);n.value=value;return n;};
let loaded=null,busy=false;
function lock(value){busy=value;for(const control of el('form').elements)control.disabled=value;el('refresh').disabled=value;el('submit').disabled=value||!loaded;}
function render(p){
  const box=el('result');box.replaceChildren();
  box.append(node('h3',`${label[p.status]} · ${date(p.created_at)}`),node('p',`所选证据：${label[p.evidence_status]} · 现金合计 ${nativeMoney(p.selected_cash_cents,'CNY')} · 未获交易许可`),node('p',`绑定研究 ${p.request.evaluation_id} · 策略 v${p.source_evaluation.strategy.revision} · ${p.context?.account.label??'未指定账户'}`));
  box.append(decisionList(p.checks,names));
}
function selections(e){
  const rows=[];
  for(const line of e.plan.lines){
    const row=node('fieldset'),legend=node('legend',line.candidate.item.name),choose=node('input');choose.type='checkbox';choose.checked=true;choose.dataset.itemKey=line.candidate.item.key;
    const choice=node('label','检查此商品 ');choice.prepend(choose);
    const count=node('input');count.type='number';count.min='1';count.max=String(line.quantity);count.value=String(line.quantity);count.required=true;
    const quantity=node('label',`数量（原计划 ${line.quantity}）`);quantity.append(count);
    choose.onchange=()=>{count.disabled=!choose.checked;};
    row.append(legend,choice,quantity,node('p',`原计划单价 ${nativeMoney(line.candidate.item.cash_cents,'CNY')} · ${line.candidate.item.key}`));rows.push(row);
  }
  el('selections').replaceChildren(...rows);
}
async function records(){
  const {items}=await parallax.preflights();
  const rows=items.map(p=>{const row=node('p'),button=node('button',`${date(p.created_at)} · ${label[p.status]} · ${nativeMoney(p.selected_cash_cents,'CNY')}`);button.type='button';button.className='secondary';
    button.onclick=async()=>{button.disabled=true;try{render(await parallax.preflight(p.id));el('feedback').textContent='已读取当时的检查记录；没有重新检查或刷新报价。';}catch(e){el('feedback').textContent=`读取失败：${e.message}`;}finally{button.disabled=false;}};row.append(button);return row;});
  el('records').replaceChildren(...(rows.length?rows:[node('p','暂无预检记录。')]));
}
async function refresh(){
  if(busy)return;lock(true);
  try{
    const results=await Promise.allSettled([parallax.strategyEvaluations(),parallax.accounts(),records()]);
    if(results[0].status==='fulfilled'){
      const items=results[0].value.items,previous=el('evaluation').value;
      const options=items.map(e=>option(e.id,`${e.strategy_name} · v${e.revision} · ${date(e.created_at)} · ${e.id.slice(0,8)}`));
      if(loaded&&!items.some(e=>e.id===loaded.id))options.push(option(loaded.id,`当前引用 · ${loaded.id.slice(0,8)}`));
      el('evaluation').replaceChildren(option('','选择研究记录'),...options);el('evaluation').value=previous;
    }
    if(results[1].status==='fulfilled'){
      const previous=el('account').value;
      el('account').replaceChildren(option('','未指定（只检查缺项）'),...results[1].value.items.map(a=>option(a.id,a.label)));el('account').value=previous;
    }
    const errors=results.filter(r=>r.status==='rejected').map(r=>r.reason.message);if(errors.length)el('feedback').textContent=`部分列表未刷新：${errors.join('；')}；当前输入保留。`;
  }finally{lock(false);restoreChoices();}
}
function restoreChoices(){for(const row of el('selections').children){const choose=row.querySelector('input[type=checkbox]');row.querySelector('input[type=number]').disabled=!choose.checked;}}
el('evaluation').onchange=async()=>{
  if(busy)return;
  const id=el('evaluation').value;
  if(!id){loaded=null;el('selections').replaceChildren();el('submit').disabled=true;return;}
  lock(true);
  try{const e=await parallax.strategyEvaluation(id);loaded=e;selections(e);el('account').value=e.request.account_id??e.plan.request.buff_account_id??'';el('feedback').textContent='已载入研究；下方可减少数量或取消商品。';}
  catch(e){loaded=null;el('feedback').textContent=`载入失败：${e.message}；原输入保留，请重新选择记录。`;}
  finally{lock(false);restoreChoices();}
};
el('form').onsubmit=async event=>{
  event.preventDefault();if(busy||!loaded)return;
  const selected=[...el('selections').children].filter(row=>row.querySelector('input[type=checkbox]').checked).map(row=>({item_key:row.querySelector('input[type=checkbox]').dataset.itemKey,quantity:Number(row.querySelector('input[type=number]').value)}));
  if(!selected.length){el('feedback').textContent='至少选择一件商品；输入与原结果保留。';return;}
  const request={evaluation_id:loaded.id,account_id:el('account').value||null,selections:selected,max_quote_age_secs:Number(el('age').value)};
  lock(true);el('feedback').textContent='正在检查本地证据并保存…';
  try{render(await parallax.createPreflight(request));el('feedback').textContent='证据检查已保存；没有发起平台请求、签发交易令牌或创建订单。';try{await records();}catch(e){el('feedback').textContent=`检查已保存，记录刷新失败：${e.message}`;}}
  catch(e){el('feedback').textContent=`本次未确认保存：${e.message}；输入与原结果保留。`;}
  finally{lock(false);restoreChoices();}
};
el('refresh').onclick=()=>refresh().catch(e=>el('feedback').textContent=e.message);
window.addEventListener('parallax:pagechange',event=>{if(event.detail.id==='strategies')refresh().catch(e=>el('feedback').textContent=e.message);});
window.addEventListener('parallax:research-saved',()=>refresh().catch(e=>el('feedback').textContent=e.message));
refresh().catch(e=>el('feedback').textContent=e.message);
