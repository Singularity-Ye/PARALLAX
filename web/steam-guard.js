import {parallax} from './lib/api.js';

const el = id => document.getElementById(`guard-${id}`);
let busy = false, ticket = 0, configured = false, expires = 0, currentCode = '';
let copyTimer = null;
const CIRCUMFERENCE = 2 * Math.PI * 110; // ~691.15

function updateRing(fraction) {
  const circle = document.getElementById('steam-progress-circle');
  if (!circle) return;
  circle.style.strokeDasharray = `${CIRCUMFERENCE}`;
  const offset = CIRCUMFERENCE * (1 - fraction);
  circle.style.strokeDashoffset = `${offset}`;
}

function clear() {
  expires = 0;
  currentCode = '';
  if (el('code')) el('code').textContent = '— — — — —';
  if (el('remaining')) el('remaining').textContent = '点击圆盘生成验证码';
  if (el('copy')) {
    el('copy').disabled = true;
    el('copy').innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true" style="margin-right:4px"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>复制验证码';
  }
  updateRing(0);
  if (copyTimer) {
    clearTimeout(copyTimer);
    copyTimer = null;
  }
}

function ready() {
  if (el('refresh')) el('refresh').disabled = busy;
  if (el('generate')) el('generate').style.pointerEvents = (busy || !configured || !el('account')?.value) ? 'none' : 'auto';
  if (el('save')) el('save').disabled = busy || !el('account')?.value;
  if (el('remove')) el('remove').disabled = busy || !configured;
  if (el('account')) el('account').disabled = busy;
  if (el('secret')) el('secret').disabled = busy;
  if (el('copy')) el('copy').disabled = busy || !currentCode || !expires;
}

async function status() {
  const current = ++ticket, account = el('account')?.value;
  clear();
  configured = false;
  ready();
  if (!account) {
    if (el('status')) el('status').textContent = '请先选择一个账户。';
    return;
  }
  try {
    const value = await parallax.guardStatus(account);
    if (current !== ticket) return;
    configured = value.configured;
    if (el('status')) {
      el('status').textContent = `Steam ID: ${value.steam_id} · ${configured ? '已配置本地密钥' : '尚未配置本地密钥'} · 点击圆盘即可刷新`;
    }
    if (el('remaining')) {
      el('remaining').textContent = configured ? '点击圆盘生成验证码' : '请先在下方抽屉配置密钥';
    }
  } catch (e) {
    if (current === ticket && el('status')) el('status').textContent = `读取失败：${e.message}`;
  } finally {
    if (current === ticket) ready();
  }
}

async function refresh() {
  const previous = el('account')?.value;
  try {
    const {items} = await parallax.accounts();
    if (el('account')) {
      el('account').replaceChildren();
      for (const item of items) {
        const o = document.createElement('option');
        o.value = item.id;
        o.textContent = item.label;
        el('account').append(o);
      }
      if (items.some(a => a.id === previous)) el('account').value = previous;
    }
    await status();
  } catch (e) {
    if (el('status')) el('status').textContent = e.message;
  }
}

if (el('account')) {
  el('account').onchange = () => {
    if (el('secret')) el('secret').value = '';
    status();
  };
}
if (el('refresh')) el('refresh').onclick = refresh;

if (el('form')) {
  el('form').onsubmit = async event => {
    event.preventDefault();
    if (busy) return;
    busy = true;
    ready();
    clear();
    const account = el('account')?.value;
    try {
      await parallax.saveGuard(account, el('secret').value.trim());
      if (el('secret')) el('secret').value = '';
      configured = true;
      if (el('feedback')) el('feedback').textContent = '已保存到本机 Windows 凭据管理器！';
      await status();
    } catch (e) {
      if (el('feedback')) el('feedback').textContent = `保存失败：${e.message}`;
    } finally {
      busy = false;
      ready();
    }
  };
}

async function generateCode() {
  if (busy || !configured || !el('account')?.value) return;
  busy = true;
  ready();
  clear();
  try {
    const value = await parallax.guardCode(el('account').value);
    if (location.hash !== '#steam-guard' || document.hidden) return;
    expires = value.valid_until;
    currentCode = value.code;
    if (el('code')) el('code').textContent = value.code;
    if (el('feedback')) el('feedback').textContent = '验证码已依据本机时钟生成，请尽快在 Steam 登录窗口输入。';
    if (el('copy')) el('copy').disabled = false;
    tick();
  } catch (e) {
    if (el('feedback')) el('feedback').textContent = e.message;
  } finally {
    busy = false;
    ready();
  }
}

if (el('generate')) {
  el('generate').onclick = generateCode;
  el('generate').onkeydown = e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); generateCode(); } };
}

async function copyCode() {
  if (!currentCode || !expires) return;
  try {
    await navigator.clipboard.writeText(currentCode);
    if (el('copy')) el('copy').innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true" style="margin-right:4px"><polyline points="20 6 9 17 4 12"></polyline></svg>已复制';
    if (el('feedback')) el('feedback').textContent = '已复制！请尽快在 Steam 官方登录界面粘贴。';
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => {
      if (el('copy') && currentCode) el('copy').innerHTML = '<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true" style="margin-right:4px"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>复制验证码';
    }, 1800);
  } catch (err) {
    if (el('feedback')) el('feedback').textContent = '复制失败，请手动选取输入。';
  }
}

if (el('copy')) el('copy').onclick = copyCode;

if (el('remove')) {
  el('remove').onclick = async () => {
    if (busy) return;
    busy = true;
    ready();
    clear();
    try {
      await parallax.removeGuard(el('account').value);
      configured = false;
      if (el('feedback')) el('feedback').textContent = '已停用并移除本机保存的令牌密钥。';
      await status();
    } catch (e) {
      if (el('feedback')) el('feedback').textContent = e.message;
    } finally {
      busy = false;
      ready();
    }
  };
}

function tick() {
  if (!expires || !currentCode) return;
  const remaining = Math.max(0, expires - Math.floor(Date.now() / 1000));
  if (el('remaining')) el('remaining').textContent = `有效倒计时 ${remaining}s · 点击复制`;
  updateRing(remaining / 30);

  const circle = document.getElementById('steam-progress-circle');
  if (remaining <= 5 && remaining > 0) {
    if (el('code')) el('code').classList.add('expiring');
    if (circle) circle.classList.add('expiring');
  } else {
    if (el('code')) el('code').classList.remove('expiring');
    if (circle) circle.classList.remove('expiring');
  }
  if (!remaining) {
    clear();
    if (el('remaining')) el('remaining').textContent = '验证码已到期，点击圆盘重新生成';
  }
}

setInterval(tick, 1000);
document.addEventListener('visibilitychange', () => {
  if (document.hidden) clear();
});
window.addEventListener('parallax:pagechange', ({detail}) => {
  if (detail.id !== 'steam-guard') {
    clear();
    if (el('secret')) el('secret').value = '';
  }
});

ready();
refresh();
