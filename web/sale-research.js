import {parallax} from './lib/api.js';
import {node,nativeMoney,exitValuationView} from './lib/pricing-view.js';
import {decisionList,assetStageLabels} from './lib/evidence-view.js';
import {populateSelect} from './lib/forms.js';
const el=id=>document.getElementById(`sale-${id}`),number=id=>Number(el(id).value);
const actions={hold:'等待 / 暂停',review_listing:'可复核挂牌条件',unknown:'证据不足',blocked:'存在阻断'};
const names={asset_stage:'资产台账状态',inventory_age:'观察时效',asset_identity:'资产身份',reported_marketable:'可交易/出售观察',steam_session:'Steam登录态元数据',history_generation:'历史凭据代次',exit_evidence:'退出估值',trend_wait:'趋势等待',wallet_return:'钱包回报阈值'};
let busy=false,account='',lots=[];
function fields(){const mode=el('mode').value;el('profit-fields').disabled=busy||mode!=='profit_protect';el('trend-fields').disabled=busy||!(mode==='trend_wait'||mode==='profit_protect'&&el('profit-trend').checked);}
function lock(value){
  busy=value;
  for(const c of el('form').elements)c.disabled=value;
  el('refresh').disabled=value;
  el('account').disabled=value;
  el('save').disabled=value||!account;
  if(el('live-refresh'))el('live-refresh').disabled=value||!account;
  if(el('live-confirm-all'))el('live-confirm-all').disabled=value||!account;
  fields();
}
function assets(){const lot=lots.find(l=>l.id===el('lot').value);populateSelect(el('asset'),lot?.assets??[],'选择批次资产',a=>`${a.asset.asset_id} · ${assetStageLabels[a.stage]} · ${nativeMoney(a.cost_cents,'CNY')}`,a=>a.asset.asset_id);}
function render(v){const box=el('result');box.replaceChildren(node('h3',actions[v.action]),node('p',`批次 ${v.lot.id} · v${v.lot.revision} · 资产 ${v.request.asset_id} · 未执行交易`),node('p',`单件采购成本 ${nativeMoney(v.unit_cost_cents,'CNY')} · 钱包估值差额 ${v.estimated_wallet_surplus_cny_cents===null?'未知':nativeMoney(v.estimated_wallet_surplus_cny_cents,'CNY')}（不代表可提现利润）`));if(v.valuation)box.append(exitValuationView(v.valuation));box.append(decisionList(v.checks,names));const list=node('ul');for(const reason of v.execution_blockers)list.append(node('li',reason));box.append(list);}

function renderLiveListings(snap){
  const box=el('live-listings-box');
  if(!box)return;
  box.replaceChildren();
  const all=[...(snap.to_confirm||[]),...(snap.active||[])];
  if(!all.length){
    const empty=node('div','当前 Steam 社区市场暂无在售或待确认饰品挂单。');
    empty.style.cssText='padding:12px 14px;background:#f8fafc;border:1px dashed #cbd5e1;border-radius:8px;color:#475569;font-size:12.5px;';
    box.append(empty);
    return;
  }
  const wrap=document.createElement('div');
  wrap.className='table-container';
  const tbl=document.createElement('table');
  tbl.className='cockpit-table';
  tbl.innerHTML=`<thead><tr><th>状态</th><th>饰品名称</th><th>Listing ID / AssetID</th><th style="text-align:right">买家支付</th><th style="text-align:right">卖家实收</th><th style="text-align:center">操作</th></tr></thead>`;
  const tbody=document.createElement('tbody');
  for(const item of all){
    const tr=document.createElement('tr');
    const tdStatus=document.createElement('td');
    tdStatus.innerHTML=item.need_confirmation
      ?'<span class="badge warn">⏳ 待令牌确认</span>'
      :'<span class="badge ok">● 在售中</span>';
    const tdName=document.createElement('td');
    tdName.textContent=item.item_name||item.market_hash_name;
    const tdId=document.createElement('td');
    tdId.style.cssText='font-family:var(--mono);font-size:11.5px;color:#64748b;';
    tdId.textContent=`L:${item.listing_id||'-'} / A:${item.asset_id||'-'}`;
    const tdBuyer=document.createElement('td');
    tdBuyer.style.textAlign='right';
    tdBuyer.textContent=`${(item.total_minor/100).toFixed(2)}`;
    const tdSeller=document.createElement('td');
    tdSeller.style.textAlign='right';
    tdSeller.textContent=`${(item.price_minor/100).toFixed(2)}`;
    const tdAct=document.createElement('td');
    tdAct.style.textAlign='center';
    if(item.listing_id){
      const btn=document.createElement('button');
      btn.type='button';
      btn.className='btn btn-sm btn-danger-outline';
      btn.textContent='撤单下架';
      btn.onclick=async()=>{
        if(busy||!account)return;
        btn.disabled=true;
        btn.textContent='撤单中...';
        try{
          await parallax.steamMarketDelist(account,item.listing_id);
          if(el('live-feedback'))el('live-feedback').textContent=`✓ 已成功从 Steam 市场撤回挂单 #${item.listing_id} (${item.item_name})！`;
          await refreshLiveListings();
        }catch(err){
          btn.disabled=false;
          btn.textContent='撤单下架';
          if(el('live-feedback'))el('live-feedback').textContent=`撤单未通过：${err.message}`;
        }
      };
      tdAct.append(btn);
    }
    tr.append(tdStatus,tdName,tdId,tdBuyer,tdSeller,tdAct);
    tbody.append(tr);
  }
  tbl.append(tbody);
  wrap.append(tbl);
  box.append(wrap);
}

async function refreshLiveListings(){
  if(!account||!el('live-listings-box'))return;
  if(el('live-feedback'))el('live-feedback').textContent='正在直连 Steam 社区市场拉取当前在售与待确认挂单...';
  try{
    const snap=await parallax.steamMarketListings(account);
    renderLiveListings(snap);
    if(el('live-feedback')){
      el('live-feedback').textContent=`✓ 已同步 Steam 实时挂单：在售 ${(snap.active||[]).length} 件 · 待令牌确认 ${(snap.to_confirm||[]).length} 件（更新于 ${new Date(snap.observed_at*1000).toLocaleTimeString()}）`;
    }
  }catch(e){
    if(el('live-feedback'))el('live-feedback').textContent=`读取 Steam 在售列表提示：${e.message}`;
  }
}

if(el('live-refresh')){
  el('live-refresh').onclick=async()=>{
    if(busy||!account)return;
    lock(true);
    try{await refreshLiveListings();}finally{lock(false);}
  };
}

if(el('live-confirm-all')){
  el('live-confirm-all').onclick=async()=>{
    if(busy||!account)return;
    lock(true);
    if(el('live-feedback'))el('live-feedback').textContent='正在调用 Steam mobileconf 一键批准所有待确认挂牌...';
    try{
      const res=await parallax.confirmGuardAll(account);
      if(el('live-feedback')){
        el('live-feedback').textContent=res.confirmed>0
          ?`⚡ 已成功免手机批准 ${res.confirmed} 项待确认市场挂牌！正在刷新在售列表...`
          :'当前没有待确认的市场挂牌（0 项）。';
      }
      await refreshLiveListings();
    }catch(e){
      if(el('live-feedback'))el('live-feedback').textContent=`一键令牌确认提示：${e.message}`;
    }finally{
      lock(false);
    }
  };
}

async function records(){if(!account)return;const {items}=await parallax.saleResearches(account);const owner=account;el('records').replaceChildren(...items.map(v=>{const button=node('button',`${new Date(v.created_at*1000).toLocaleString()} · ${actions[v.action]} · 资产 ${v.asset_id}`);button.type='button';button.className='secondary';button.onclick=async()=>{if(busy)return;lock(true);try{render(await parallax.saleResearch(owner,v.id));el('feedback').textContent='已读取当时的出售研究，未刷新或重算。';}catch(e){el('feedback').textContent=e.message;}finally{lock(false);}};return button;}));if(!items.length)el('records').append(node('p','暂无出售研究。'));}
async function refresh(){if(busy)return;lock(true);try{
  const {items:accounts}=await parallax.accounts();populateSelect(el('account'),accounts,'选择研究账户',a=>a.label);if(!el('account').value&&accounts.length>=1)el('account').value=accounts[0].id;
  const next=el('account').value;if(next!==account){account=next;lots=[];for(const id of ['lot','asset','inventory','evaluation','records'])el(id).replaceChildren();}
  if(!account){el('feedback').textContent='先选择已有账户和采购批次；此页不会创建模拟资产。';return;}
  const results=await Promise.allSettled([parallax.purchaseLots(account),parallax.inventories(account),parallax.strategyEvaluations(),records(),refreshLiveListings()]);
  if(results[0].status==='fulfilled'){lots=results[0].value.items;populateSelect(el('lot'),lots,'选择采购批次',l=>`${l.request.item_key} · v${l.revision}`);assets();}
  if(results[1].status==='fulfilled')populateSelect(el('inventory'),results[1].value.items,'选择显式库存观察',i=>`${new Date(i.observed_at*1000).toLocaleString()} · ${i.id.slice(0,8)}`);
  if(results[2].status==='fulfilled')populateSelect(el('evaluation'),results[2].value.items,'选择已有研究',e=>`${e.strategy_name} · v${e.revision} · ${e.id.slice(0,8)}`);
  const errors=results.slice(0,4).filter(r=>r.status==='rejected').map(r=>r.reason.message);if(errors.length)el('feedback').textContent=`部分列表未刷新：${errors.join('；')}；输入保留。`;
}catch(e){el('feedback').textContent=`刷新失败：${e.message}；原结果保留。`;}finally{lock(false);}}
el('form').onsubmit=async event=>{event.preventDefault();if(busy||!account)return;const mode=el('mode').value,gate={min_points:number('points'),min_r_squared_bps:number('r2'),rising_above_bps_per_day:number('slope')};let policy={mode};if(mode==='trend_wait')policy.gate=gate;if(mode==='profit_protect')Object.assign(policy,{min_wallet_return_bps:number('return'),basis:el('basis').value,trend:el('profit-trend').checked?gate:null});const request={lot_id:el('lot').value,asset_id:el('asset').value,inventory_snapshot_id:el('inventory').value,evaluation_id:el('evaluation').value,max_inventory_age_secs:number('age'),policy};lock(true);el('feedback').textContent='正在按当前证据与分摊成本研究…';try{render(await parallax.createSaleResearch(account,request));el('feedback').textContent='出售研究已保存，未调用挂牌/撤单/成交平台接口。';try{await records();}catch(e){el('feedback').textContent=`研究已保存，记录刷新失败：${e.message}`;}}catch(e){el('feedback').textContent=`本次未确认保存：${e.message}；输入与旧结果保留。`;}finally{lock(false);}};
el('lot').onchange=assets;el('mode').onchange=fields;el('profit-trend').onchange=fields;el('refresh').onclick=refresh;
el('account').onchange=()=>{account='';el('result').replaceChildren();refresh();};
window.addEventListener('parallax:pagechange',event=>{if(event.detail.id==='sales')refresh();});window.addEventListener('parallax:ledger-changed',refresh);window.addEventListener('parallax:account-created',refresh);
refresh();
