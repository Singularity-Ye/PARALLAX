import { parallax } from './lib/api.js';
const el = id => document.getElementById(id);
const select = el('account-select');
const platform = el('session-platform');
const form = el('session-form');
const states = {unverified:'尚未验证',valid:'验证有效',invalid:'失效或身份不符',unknown:'无法确认',rate_limited:'限流冷却',verification_required:'需浏览器验证',missing:'本地凭据已停用或缺失'};
async function sessions() {
  el('session-list').replaceChildren();
  if (!select.value) return;
  const {items} = await parallax.sessions(select.value);
  for (const s of items) {
    const row = document.createElement('p');
    row.textContent = `${s.platform.toUpperCase()} · ${states[s.status] || s.status} · ${s.message}${s.checked_at ? ' · 检查于 '+new Date(s.checked_at*1000).toLocaleString() : ''}${s.retry_at>Date.now()/1000 ? ' · 下次可检查 '+new Date(s.retry_at*1000).toLocaleTimeString() : ''}`;
    el('session-list').append(row);
  }
}
async function loadAccounts(preferred) {
  const {items} = await parallax.accounts();
  select.replaceChildren();
  for (const a of items) { const o = document.createElement('option');o.value=a.id;o.textContent=`${a.label} · ${a.steam_id}`;select.append(o); }
  if (preferred) select.value = preferred;
  await sessions();
}
async function action(fn) {
  const controls = [...document.querySelectorAll('#accounts button, #accounts select')];
  controls.forEach(c=>c.disabled=true);
  el('account-feedback').textContent='处理中…';
  try { el('account-feedback').textContent = await fn() || '已完成'; }
  catch(e) { el('account-feedback').textContent=e.message; }
  finally { controls.forEach(c=>c.disabled=false); }
}
el('account-form').onsubmit = e => {e.preventDefault();action(async()=>{
  const values = Object.fromEntries(new FormData(e.target));
  const account = await parallax.createAccount(values);
  e.target.reset();await loadAccounts(account.id);return '账户已创建';
});};
form.onsubmit = e => {e.preventDefault();action(async()=>{
  if (!select.value) throw new Error('请先创建账户');
  const saved = await parallax.importSession(select.value,platform.value,Object.fromEntries(new FormData(form)));
  form.elements.cookies.value='';await sessions();return saved.message;
});};
el('session-verify').onclick = ()=>action(async()=>{
  if (!select.value) throw new Error('请先创建账户');
  const result = await parallax.verifySession(select.value,platform.value);await sessions();return result.message;
});
el('session-revoke').onclick = ()=>action(async()=>{
  if (!select.value) throw new Error('请先创建账户');
  const result = await parallax.revokeSession(select.value,platform.value);await sessions();return result.message;
});
select.onchange=()=>action(sessions);
loadAccounts().catch(e=>el('account-feedback').textContent=e.message);
