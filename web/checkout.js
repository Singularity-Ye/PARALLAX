import {parallax} from './lib/api.js';
import {node,nativeMoney} from './lib/pricing-view.js';
import {populateSelect,localTime,unixTime,retryableCommand} from './lib/forms.js';
import {rNode,renderKpiGrid} from './lib/research-view.js';

const el=id=>document.getElementById(`checkout-${id}`),cash=n=>nativeMoney(n,'CNY');
const states={reserved:'本地已预留',cancelled:'本地已取消'};
const fundingCommand=retryableCommand(),prepareCommand=retryableCommand(),cancelCommand=retryableCommand();
let account='',funds=null,selected=null,busy=false;
el('observed').value=localTime();

function lock(value){busy=value;for(const id of ['funding-form','prepare-form','cancel-form'])for(const c of el(id).elements)c.disabled=value;for(const id of ['account','refresh','events'])el(id).disabled=value;ready();}
function ready(){el('observe').disabled=busy||!account;el('prepare').disabled=busy||!account||!funds?.snapshot;el('cancel').disabled=busy||selected?.state!=='reserved';el('events').disabled=busy||!selected;}

function renderFunds(v){
  funds=v;
  const box=el('funds');
  const kpis=renderKpiGrid([
    {label:`BUFF 主体 (${v.principal})`,value:v.snapshot?(v.fresh?'观察有效':'观察已过期'):'未登记'},
    {label:'已预留额度',value:cash(v.reserved_cash_cents)},
    {label:'剩余参考现金',value:v.available_after_reservations_cents===null?'未知':cash(v.available_after_reservations_cents),tone:'good'},
  ]);
  box.replaceChildren(kpis);
  if(v.snapshot) {
    const p=rNode('p','hint',`登记时间 ${new Date(v.snapshot.request.observed_at*1000).toLocaleString()} · 凭据 ${v.snapshot.request.evidence_ref}`);
    p.style.marginTop='6px';
    box.append(p);
  }
  if(v.deficit_cents) {
    const d=rNode('p','hint',`观察金额低于已预留额度，缺口 ${cash(v.deficit_cents)}；没有自动释放意图。`);
    d.style.color='#a3433d';
    box.append(d);
  }
  ready();
}

function renderIntent(v){
  selected=v;el('event-list').replaceChildren();
  const box=el('result');
  const card=rNode('div','research-panel-card');
  const head=rNode('div','research-card-head');
  const badge=rNode('span',`badge ${v.state==='reserved'?'ok':'muted'}`,`${states[v.state]} · v${v.revision}`);
  head.append(rNode('h4','',`意图 ${v.id.slice(0,8)} · 金额 ${cash(v.reserved_cash_cents)}`),badge);
  card.append(head,rNode('p','hint',`引用预检 ${v.request.preflight_id.slice(0,8)} · 资金凭据 ${v.funding_snapshot.request.evidence_ref} · ${v.state==='reserved'?'额度继续保留':'额度已释放'}`));
  const items=node('ul');
  for(const s of v.rechecked_preflight.request.selections)items.append(node('li',`${s.item_key} · ${s.quantity} 件`));
  card.append(items);
  box.replaceChildren(card);
  ready();
}

async function records(){
  const {items}=await parallax.purchaseIntents(account),owner=account;
  const rows=items.map(v=>{
    const b=node('button');b.type='button';b.className='history-row';
    b.append(node('strong',`${states[v.state]} · ${cash(v.reserved_cash_cents)}`),node('span',new Date(v.created_at*1000).toLocaleDateString()));
    b.onclick=async()=>{if(busy)return;lock(true);try{renderIntent(await parallax.purchaseIntent(owner,v.id));el('feedback').textContent='已恢复保存的本地意图，没有重新预留或提交平台。';}catch(e){el('feedback').textContent=e.message;}finally{lock(false);}};
    return b;
  });
  el('records').replaceChildren(...(rows.length?rows:[rNode('p','hint','暂无本地采购意图。')]));
}

async function data(){
  if(!account)return;
  const result=await Promise.allSettled([parallax.cashFunding(account),parallax.preflights(),records()]);
  if(result[0].status==='fulfilled')renderFunds(result[0].value);else funds=null;
  if(result[1].status==='fulfilled')populateSelect(el('preflight'),result[1].value.items.filter(p=>p.account_id===account),'选择账户预检',p=>`${new Date(p.created_at*1000).toLocaleString()} · ${cash(p.selected_cash_cents)} · ${p.id.slice(0,8)}`);
  const errors=result.filter(r=>r.status==='rejected').map(r=>r.reason.message);
  if(errors.length)throw new Error(errors.join('；'));
}

async function refresh(){
  if(busy)return;lock(true);
  try{
    const {items}=await parallax.accounts();populateSelect(el('account'),items,'选择准备账户',a=>a.label);
    if(!el('account').value&&items.length===1)el('account').value=items[0].id;
    const next=el('account').value;
    if(next!==account){account=next;funds=null;selected=null;for(const id of ['funds','records','result','preflight','event-list'])el(id).replaceChildren();}
    if(account)await data();else el('feedback').textContent='先选择已有账户；资金归属需要最近核验的 BUFF 身份。';
  }catch(e){el('feedback').textContent=`部分数据未刷新：${e.message}；表单及原结果保留。`;}
  finally{lock(false);}
}

async function write(run,onSuccess){
  if(busy)return;lock(true);
  try{
    const result=await run();onSuccess(result.item);
    el('feedback').textContent=result.replayed?'已恢复相同请求的记录，没有重复占用资金。':'本地记录已保存，没有向平台发请求。';
    try{await data();}catch(e){el('feedback').textContent=`记录已保存，部分列表刷新失败：${e.message}`;}
  }catch(e){el('feedback').textContent=`本次未确认保存：${e.message}；输入和原结果保留。`;}
  finally{lock(false);}
}

el('funding-form').onsubmit=e=>{e.preventDefault();let r;try{r=fundingCommand({available_cash_cents:Number(el('cash').value),observed_at:unixTime(el('observed')),evidence_ref:el('proof').value},account);}catch(e){el('feedback').textContent=e.message;return;}write(()=>parallax.observeCashFunding(account,r),v=>el('feedback').textContent=`资金观察 ${v.id} 已保存`);};
el('prepare-form').onsubmit=e=>{e.preventDefault();if(!funds?.snapshot)return;const r=prepareCommand({preflight_id:el('preflight').value,funding_snapshot_id:funds.snapshot.id,max_funding_age_secs:Number(el('age').value)},account);write(()=>parallax.preparePurchaseIntent(account,r),renderIntent);};
el('cancel-form').onsubmit=e=>{e.preventDefault();if(!selected)return;const r=cancelCommand({expected_revision:selected.revision,reason:el('reason').value},`${account}/${selected.id}`);write(()=>parallax.cancelPurchaseIntent(account,selected.id,r),renderIntent);};
el('events').onclick=async()=>{if(busy||!selected)return;lock(true);try{const {items}=await parallax.purchaseIntentEvents(account,selected.id);el('event-list').replaceChildren(...items.map(e=>rNode('p','hint',`v${e.result_revision} · 本地取消 · ${new Date(e.recorded_at*1000).toLocaleString()} · ${e.request.reason}`)));if(!items.length)el('event-list').append(rNode('p','hint','没有取消事件；准备依据保存在意图内。'));}catch(e){el('feedback').textContent=e.message;}finally{lock(false);}};
el('account').onchange=()=>refresh();el('refresh').onclick=refresh;
window.addEventListener('parallax:pagechange',e=>{if(e.detail.id==='purchase-history')refresh();});
window.addEventListener('parallax:account-created',refresh);
refresh();
