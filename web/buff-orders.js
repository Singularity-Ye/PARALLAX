import {parallax} from './lib/api.js';
import {node} from './lib/pricing-view.js';
import {populateSelect} from './lib/forms.js';

const el=id=>document.getElementById(`buff-orders-${id}`);
const stops={short_page:'遇到短页',reported_end:'达到上游报告条数',page_budget:'达到本次页数上限'};
let account='',snapshot=null,busy=false;
function lock(value){
  busy=value;
  for(const id of ['account','refresh','fetch','max-pages','page-size','saved','open','order-id','lookup'])el(id).disabled=value;
  el('fetch').disabled=value||!account;
  el('open').disabled=value||!account||!el('saved').value;
  el('lookup').disabled=value||!snapshot;
}
function render(value){
  snapshot=value;
  el('lookup-result').replaceChildren();
  const box=el('result'),count=value.pages.reduce((n,p)=>n+p.orders.length,0);
  box.replaceChildren(
    node('h3',`保存的订单观察 · ${count} 条`),
    node('p',`BUFF主体 ${value.principal} · ${new Date(value.started_at*1000).toLocaleString()} 至 ${new Date(value.observed_at*1000).toLocaleString()}`),
    node('p',`${value.pages.length} 页 · ${stops[value.stop_reason]} · 分页并非同一时刻的完整订单清单，不能证明缺失订单不存在。`),
    node('p','以下为保存时的平台状态码，尚未完成真实账户付款语义验收。不会据此释放资金、登记成交或重试下单。'),
  );
  const table=document.createElement('table'),head=document.createElement('thead'),row=document.createElement('tr');
  for(const title of ['页','准确订单号','保存时状态','支付窗口 / 秒'])row.append(node('th',title));
  head.append(row);table.append(head);
  const body=document.createElement('tbody');
  for(const page of value.pages)for(const order of page.orders){
    const r=document.createElement('tr'),select=node('button',order.order_id);
    select.type='button';select.className='secondary';
    select.onclick=()=>{if(!busy){el('order-id').value=order.order_id;lookup();}};
    const id=document.createElement('td');id.append(select);
    r.append(node('td',String(page.page_num)),id,node('td',order.state_code===null?'未知':String(order.state_code)),node('td',order.pay_expire_timeout_secs===null?'未知':String(order.pay_expire_timeout_secs)));
    body.append(r);
  }
  table.append(body);box.append(table);if(!count)box.append(node('p','本次页面为空；读取失败不会保存成空记录。'));
}
async function saved(){
  const {items}=await parallax.buffOrderObservations(account);
  populateSelect(el('saved'),items,'选择保存的订单观察',s=>`${new Date(s.observed_at*1000).toLocaleString()} · ${s.id.slice(0,8)}`);
}
async function refresh(){
  if(busy)return;lock(true);
  try{
    const {items}=await parallax.accounts();populateSelect(el('account'),items,'选择订单账户',a=>a.label);
    if(!el('account').value&&items.length===1)el('account').value=items[0].id;
    if(account!==el('account').value){account=el('account').value;snapshot=null;for(const id of ['result','lookup-result','saved'])el(id).replaceChildren();}
    if(account)await saved();else el('feedback').textContent='先选择已有账户；读取需要最近核验的 BUFF 登录态。';
  }catch(e){el('feedback').textContent=`列表未刷新：${e.message}；表单及原结果保留。`;}
  finally{lock(false);}
}
async function lookup(){
  if(busy||!snapshot)return;lock(true);
  try{
    const result=await parallax.lookupBuffOrder(account,snapshot.id,{order_id:el('order-id').value});
    el('lookup-result').replaceChildren(node('h3',result.found?'在已观察页面找到订单':'本次页面未找到订单'),node('p',result.reason),node('p',`观察距今 ${result.observation_age_secs===null?'未知':`${result.observation_age_secs} 秒`}，未获得交易许可。`));
    el('feedback').textContent='仅查保存的快照，没有访问平台或修改资金/台账。';
  }catch(e){el('feedback').textContent=e.message;}
  finally{lock(false);}
}
el('fetch-form').onsubmit=async e=>{
  e.preventDefault();if(busy||!account)return;lock(true);el('feedback').textContent='正在只读观察；失败时保留旧快照。';
  try{
    render(await parallax.observeBuffOrders(account,{max_pages:Number(el('max-pages').value),page_size:Number(el('page-size').value)}));
    el('feedback').textContent='新订单观察已保存，没有创建订单或核销付款。';
    try{await saved();el('saved').value=snapshot.id;}catch(e){el('feedback').textContent=`观察已保存，但列表刷新失败：${e.message}`;}
  }catch(e){el('feedback').textContent=`未保存新观察：${e.message}；输入和原结果保留。`;}
  finally{lock(false);}
};
el('open').onclick=async()=>{
  if(busy||!el('saved').value)return;lock(true);
  try{render(await parallax.buffOrderObservation(account,el('saved').value));el('feedback').textContent='已恢复当时订单观察，没有刷新平台状态。';}
  catch(e){el('feedback').textContent=e.message;}finally{lock(false);}
};
el('lookup-form').onsubmit=e=>{e.preventDefault();lookup();};
el('account').onchange=refresh;el('refresh').onclick=refresh;el('saved').onchange=()=>lock(false);
window.addEventListener('parallax:pagechange',e=>{if(e.detail.id==='purchase-history')refresh();});
window.addEventListener('parallax:account-created',refresh);refresh();
