import {parallax} from './lib/api.js';
import {node,nativeMoney} from './lib/pricing-view.js';
import {currencyOptions} from './lib/currencies.js';
import {populateSelect,localTime,unixTime,retryableCommand} from './lib/forms.js';
import {assetStageLabels as stages} from './lib/evidence-view.js';
const el=id=>document.getElementById(`ledger-${id}`),cash=n=>nativeMoney(n,'CNY');
const actionLabels={receive:'收货绑定',list:'挂牌登记',cancel_listing:'撤单登记',sell:'成交登记'};
let busy=false,loaded=null,inventory=null,account='',selectionContext='';
const createCommand=retryableCommand(),eventCommand=retryableCommand(),receiveCommand=retryableCommand();
el('purchased-at').value=localTime();el('event-at').value=localTime();currencyOptions(el('currency'));
function fields(){const kind=el('action').value;el('sale-ref').disabled=busy||kind!=='sell';el('sale-ref').required=kind==='sell';for(const id of ['amount','currency'])el(id).disabled=busy||kind==='cancel_listing';el('amount').required=kind!=='cancel_listing';}
function lock(value){busy=value;for(const id of ['create-form','receive-form','event-form'])for(const field of el(id).elements)field.disabled=value;for(const id of ['account','lot','inventory','refresh','events','holdings'])el(id).disabled=value;ready();}
function ready(){el('create').disabled=busy||!account;el('receive').disabled=busy||!loaded||!inventory;el('event').disabled=busy||!loaded;el('events').disabled=busy||!loaded;el('holdings').disabled=busy||!account;fields();}
function showLot(lot){
  loaded=lot;const box=el('lot-result');box.replaceChildren(node('h3',`${lot.request.item_key} · v${lot.revision}`),node('p',`批次 ${lot.id} · 人工未核验 · ${lot.request.quantity} 件 · 整批成本 ${cash(lot.request.cash_cost_cents)}`));
  const list=node('ul');for(const a of lot.assets)list.append(node('li',`资产 ${a.asset.asset_id} · ${stages[a.stage]} · 分摊成本 ${cash(a.cost_cents)}${a.sale?` · 已报净钱包 ${nativeMoney(a.sale.net_wallet.minor,a.sale.net_wallet.currency)}`:''}`));box.append(list);
  populateSelect(el('asset'),lot.assets,'选择资产',a=>`${a.asset.asset_id} · ${stages[a.stage]}`,a=>a.asset.asset_id);receiveChoices();ready();
}
function receiveChoices(){
  const host=el('receive-assets');
  if(!loaded||!inventory){host.replaceChildren(node('p','先选择批次和库存观察。'));return;}
  const context=`${account}/${loaded.id}/${inventory.id}`,previous=new Set(context===selectionContext?[...host.querySelectorAll('input:checked')].map(i=>i.value):[]);selectionContext=context;
  const rows=inventory.assets.filter(a=>a.context_id==='2'&&a.amount===1&&a.market_hash_name===loaded.request.item_key&&!loaded.assets.some(x=>x.asset.asset_id===a.asset_id)).map(a=>{const label=node('label',`资产 ${a.asset_id} · class ${a.class_id} / instance ${a.instance_id}`),check=node('input');check.type='checkbox';check.value=a.asset_id;check.checked=previous.has(a.asset_id);label.prepend(check);return label;});
  host.replaceChildren(...(rows.length?rows:[node('p','这份观察没有可绑定的单件精确身份资产；不按名称自动猜测。')]));
}
async function loadLists(){
  if(!account){el('feedback').textContent='请先在账号管理创建或选择账户；此页不会创建模拟账户。';return;}
  const result=await Promise.allSettled([parallax.purchaseLots(account),parallax.inventories(account)]);
  if(result[0].status==='fulfilled'){const items=result[0].value.items;populateSelect(el('lot'),items,'选择采购批次',l=>`${l.request.item_key} · v${l.revision} · ${l.id.slice(0,8)}`);if(loaded&&items.some(l=>l.id===loaded.id))showLot(items.find(l=>l.id===loaded.id));}
  if(result[1].status==='fulfilled')populateSelect(el('inventory'),result[1].value.items,'不引用库存观察',i=>`${new Date(i.observed_at*1000).toLocaleString()} · ${i.assets.length}资产 · ${i.id.slice(0,8)}`);
  const errors=result.filter(r=>r.status==='rejected').map(r=>r.reason.message);if(errors.length)throw new Error(errors.join('；'));
}
async function refresh(){if(busy)return;lock(true);try{const {items}=await parallax.accounts();populateSelect(el('account'),items,'选择台账账户',a=>a.label);if(!el('account').value&&items.length===1)el('account').value=items[0].id;const next=el('account').value;if(account!==next){account=next;loaded=null;inventory=null;el('lot-result').replaceChildren();el('receive-assets').replaceChildren();}await loadLists();}catch(e){el('feedback').textContent=`列表未完整刷新：${e.message}；输入保留。`;}finally{lock(false);}}
el('account').onchange=()=>{account=el('account').value;loaded=null;inventory=null;el('lot-result').replaceChildren();el('event-history').replaceChildren();el('holdings-result').replaceChildren();el('lot').replaceChildren();el('inventory').replaceChildren();receiveChoices();refresh();};
el('lot').onchange=async()=>{if(busy)return;if(!el('lot').value){loaded=null;el('lot-result').replaceChildren();receiveChoices();ready();return;}lock(true);try{showLot(await parallax.purchaseLot(account,el('lot').value));el('event-history').replaceChildren();}catch(e){loaded=null;el('feedback').textContent=e.message;}finally{lock(false);}};
el('inventory').onchange=async()=>{if(busy)return;if(!el('inventory').value){inventory=null;receiveChoices();ready();return;}lock(true);try{inventory=await parallax.inventory(account,el('inventory').value);receiveChoices();}catch(e){inventory=null;el('feedback').textContent=e.message;}finally{lock(false);}};
async function write(run){if(busy)return;lock(true);el('feedback').textContent='正在保存人工凭据登记…';try{const value=await run();showLot(value.lot);el('feedback').textContent=value.replayed?'这份输入已处理，已读取当前批次；未重复记账。':'登记已保存，未调用交易平台。';window.dispatchEvent(new CustomEvent('parallax:ledger-changed',{detail:{account}}));try{await loadLists();el('lot').value=value.lot.id;}catch(e){el('feedback').textContent=`登记已保存，列表刷新失败：${e.message}`;}}catch(e){el('feedback').textContent=`本次未确认登记：${e.message}；输入和原结果保留。`;}finally{lock(false);}}
el('create-form').onsubmit=event=>{event.preventDefault();let request;try{request=createCommand({item_key:el('item').value,quantity:Number(el('quantity').value),cash_cost_cents:Number(el('cost').value),purchased_at:unixTime(el('purchased-at')),purchase_ref:el('purchase-ref').value});}catch(e){el('feedback').textContent=e.message;return;}write(()=>parallax.createPurchaseLot(account,request));};
el('receive-form').onsubmit=event=>{event.preventDefault();if(!loaded||!inventory)return;const ids=[...el('receive-assets').querySelectorAll('input:checked')].map(i=>i.value);if(!ids.length){el('feedback').textContent='请选择明确的收货资产；原结果保留。';return;}const request=receiveCommand({expected_revision:loaded.revision,action:{kind:'receive',inventory_snapshot_id:inventory.id,asset_ids:ids}});write(async()=>{const value=await parallax.appendLotEvent(account,loaded.id,request);el('event-at').value=localTime();return value;});};
el('event-form').onsubmit=event=>{
  event.preventDefault();if(!loaded)return;
  let action;try{const kind=el('action').value,time=unixTime(el('event-at'));action={kind,asset_id:el('asset').value,listing_ref:el('listing-ref').value};if(kind==='list')Object.assign(action,{gross:{minor:Number(el('amount').value),currency:el('currency').value},listed_at:time});if(kind==='cancel_listing')action.cancelled_at=time;if(kind==='sell')Object.assign(action,{sale_ref:el('sale-ref').value,net_wallet:{minor:Number(el('amount').value),currency:el('currency').value},sold_at:time});}catch(e){el('feedback').textContent=e.message;return;}
  const request=eventCommand({expected_revision:loaded.revision,action});write(()=>parallax.appendLotEvent(account,loaded.id,request));
};
el('events').onclick=async()=>{if(busy||!loaded)return;lock(true);try{const {items}=await parallax.lotEvents(account,loaded.id);el('event-history').replaceChildren(...items.map(e=>{const d=node('details');d.append(node('summary',`v${e.result_revision} · ${actionLabels[e.request.action.kind]} · ${new Date(e.recorded_at*1000).toLocaleString()}`),node('pre',JSON.stringify(e.request.action,null,2)));return d;}));if(!items.length)el('event-history').append(node('p','尚无状态事件，采购原始凭据保留在批次中。'));}catch(e){el('feedback').textContent=e.message;}finally{lock(false);}};
el('holdings').onclick=async()=>{if(busy||!account)return;lock(true);try{const h=await parallax.holdingsPreview(account,{inventory_snapshot_id:el('inventory').value||null}),box=el('holdings-result');box.replaceChildren(node('h3',`已登记成本：未结 ${cash(h.open_cost_cents)} · 已售 ${cash(h.sold_cost_cents)}`));for(const w of h.warnings)box.append(node('p',w));for(const m of h.reported_wallet_receipts)box.append(node('p',`已报净钱包 ${nativeMoney(m.minor,m.currency)}`));for(const row of h.items){const d=node('section');d.className='candidate';d.append(node('h3',row.item_key),node('p',`未收 ${row.pending_units} · 持有 ${row.held_units} · 已挂 ${row.listed_units} · 已售 ${row.sold_units} · 观察未入账 ${row.observed_untracked_units}`),node('p',`未结成本 ${cash(row.open_cost_cents)} · 已售成本 ${cash(row.sold_cost_cents)}`));for(const conflict of row.discrepancies)d.append(node('p',conflict));box.append(d);}el('feedback').textContent='汇总已生成；仍为不完整/未核验持仓，没有改写台账。';}catch(e){el('feedback').textContent=`汇总失败：${e.message}；原结果保留。`;}finally{lock(false);}};
el('action').onchange=fields;el('refresh').onclick=refresh;
window.addEventListener('parallax:pagechange',e=>{if(e.detail.id==='purchases')refresh();});window.addEventListener('parallax:account-created',refresh);
refresh();
