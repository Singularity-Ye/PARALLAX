import { parallax } from './lib/api.js';
const el=id=>document.getElementById(id);
let renderedId=null, entries=[];
const cents=value=>Math.round(Number(value)*100);
function input(labelText,value) {
  const label=document.createElement('label');label.textContent=labelText;
  const field=document.createElement('input');field.type='number';field.min='0';field.step='1';field.max='4294967295';
  if (value!==undefined) {field.value=String(value);field.required=true;}
  label.append(field);return {label,field};
}
export function renderPositionInputs(report) {
  if (report.status!=='completed' || renderedId===report.id) return;
  renderedId=report.id;entries=[];el('position-entries').replaceChildren();
  el('use-positions').checked=false;el('position-fields').disabled=true;
  for (const candidate of report.candidates.filter(c=>c.passed)) {
    const box=document.createElement('section');box.className='candidate';
    const title=document.createElement('strong');title.textContent=candidate.item.name;box.append(title);
    const held=input('持有件数',0),pending=input('未决买入件数',0),available=input('当前可买件数（留空未知）'),daily=input('日成交量（留空未知）');
    const first=document.createElement('div'),second=document.createElement('div');first.className=second.className='pair';
    first.append(held.label,pending.label);second.append(available.label,daily.label);box.append(first,second);
    entries.push({key:candidate.item.key,held:held.field,pending:pending.field,available:available.field,daily:daily.field});
    el('position-entries').append(box);
  }
}
export function readWalletPolicy(form) {
  if (form.get('wallet_policy')!=='parameterized_fees') return {kind:'legacy115'};
  return {kind:'parameterized_fees',schedule:{currency:'CNY',platform_bps:cents(form.get('platform_rate')),publisher_bps:cents(form.get('publisher_rate')),
    platform_min_minor:cents(form.get('platform_min')),publisher_min_minor:cents(form.get('publisher_min')),increment_minor:cents(form.get('fee_increment'))}};
}
export function readPositionInputs() {
  if (!el('use-positions').checked) return {};
  const optional=field=>field.value.trim()===''?null:Number(field.value);
  const time=new Date(el('position-time').value).getTime();
  if (!el('position-account').value || !Number.isFinite(time)) throw new Error('请选择账户并填写仓位观察时间');
  return {position_snapshot:{account_id:el('position-account').value,observed_at:Math.floor(time/1000),reserved_cash_cents:cents(el('position-reserved').value),
    entries:entries.map(e=>({key:e.key,held_units:Number(e.held.value),pending_buy_units:Number(e.pending.value),available_units:optional(e.available),daily_volume:optional(e.daily)}))},
    max_position_age_secs:Number(el('position-age').value),
    liquidity_policy:el('use-liquidity').checked?{participation_bps:cents(el('daily-participation').value),low_price_threshold_cents:cents(el('low-threshold').value),low_price_penalty_bps:cents(el('low-penalty').value),low_price_cap:Number(el('low-cap').value)}:null};
}
async function refreshAccounts() {
  const selected=el('position-account').value;
  const {items}=await parallax.accounts();
  el('position-account').replaceChildren();
  for(const a of items) {const option=document.createElement('option');option.value=a.id;option.textContent=`${a.label} · ${a.steam_id}`;el('position-account').append(option);}
  if(items.some(a=>a.id===selected)) el('position-account').value=selected;
}
document.querySelector('[name="wallet_policy"]').onchange=e=>el('fee-fields').disabled=e.target.value!=='parameterized_fees';
el('use-positions').onchange=e=>el('position-fields').disabled=!e.target.checked;
el('use-liquidity').onchange=e=>el('liquidity-fields').disabled=!e.target.checked;
const loadAccounts=()=>refreshAccounts().catch(e=>el('plan-feedback').textContent=`仓位账户读取失败：${e.message}`);
el('position-account').onfocus=loadAccounts;
window.addEventListener('parallax:account-created',loadAccounts);
loadAccounts();
