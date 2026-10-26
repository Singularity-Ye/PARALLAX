import {parallax} from './lib/api.js';

const el = id => document.getElementById(`guard-${id}`);
let busy = false, ticket = 0, configured = false, expires = 0, currentCode = '';
let copyTimer = null;

function clear() {
  expires = 0;
  currentCode = '';
  el('code').textContent = '— — — — —';
  el('code').classList.remove('expiring');
  el('progress').classList.remove('expiring');
  el('remaining').textContent = '未生成验证码';
  el('progress').value = 0;
  if (el('copy')) {
    el('copy').disabled = true;
    el('copy').textContent = '复制验证码';
  }
  if (copyTimer) {
    clearTimeout(copyTimer);
    copyTimer = null;
  }
}

function ready() {
  el('refresh').disabled = busy;
  el('generate').disabled = busy || !configured || !el('account').value;
  el('save').disabled = busy || !el('account').value;
  el('remove').disabled = busy || !configured;
  el('account').disabled = busy;
  el('secret').disabled = busy;
  if (el('copy')) el('copy').disabled = busy || !currentCode || !expires;
}

async function status() {
  const current = ++ticket, account = el('account').value;
  clear();
  configured = false;
  ready();
  if (!account) {
    el('status').textContent = '先选择账户。';
    return;
  }
  try {
    const value = await parallax.guardStatus(account);
    if (current !== ticket) return;
    configured = value.configured;
    el('status').textContent = `Steam ID ${value.steam_id} · ${configured ? '已配置本地密钥' : '尚未配置'} · 仅生成登录验证码，密码自动登录/续期尚未接入。`;
  } catch (e) {
    if (current === ticket) el('status').textContent = `读取失败：${e.message}`;
  } finally {
    if (current === ticket) ready();
  }
}

async function refresh() {
  const previous = el('account').value;
  try {
    const {items} = await parallax.accounts();
    el('account').replaceChildren();
    for (const item of items) {
      const o = document.createElement('option');
      o.value = item.id;
      o.textContent = item.label;
      el('account').append(o);
    }
    if (items.some(a => a.id === previous)) el('account').value = previous;
    await status();
  } catch (e) {
    el('status').textContent = e.message;
  }
}

el('account').onchange = () => {
  el('secret').value = '';
  status();
};
el('refresh').onclick = refresh;

el('form').onsubmit = async event => {
  event.preventDefault();
  if (busy) return;
  busy = true;
  ready();
  clear();
  const account = el('account').value;
  try {
    await parallax.saveGuard(account, el('secret').value.trim());
    el('secret').value = '';
    configured = true;
    el('feedback').textContent = '已保存到本机Windows凭据管理器，未把密钥写入数据库或验证Steam登录。';
    await status();
  } catch (e) {
    el('feedback').textContent = `保存未完成：${e.message}；输入保留。`;
  } finally {
    busy = false;
    ready();
  }
};

el('generate').onclick = async () => {
  if (busy) return;
  busy = true;
  ready();
  clear();
  try {
    const value = await parallax.guardCode(el('account').value);
    if (location.hash !== '#steam-guard' || document.hidden) return;
    expires = value.valid_until;
    currentCode = value.code;
    el('code').textContent = value.code;
    el('feedback').textContent = '验证码依据本机时钟，仅用于你在Steam官方登录页面输入；不代表已登录成功。';
    if (el('copy')) el('copy').disabled = false;
    tick();
  } catch (e) {
    el('feedback').textContent = e.message;
  } finally {
    busy = false;
    ready();
  }
};

if (el('copy')) {
  el('copy').onclick = async () => {
    if (!currentCode || !expires) return;
    try {
      await navigator.clipboard.writeText(currentCode);
      el('copy').textContent = '已复制 ✓';
      el('feedback').textContent = '已复制到剪贴板，请尽快在 Steam 登录窗口输入。';
      if (copyTimer) clearTimeout(copyTimer);
      copyTimer = setTimeout(() => {
        if (el('copy') && currentCode) el('copy').textContent = '复制验证码';
      }, 1500);
    } catch (err) {
      el('feedback').textContent = '复制失败，请手动选取输入。';
    }
  };
}

el('remove').onclick = async () => {
  if (busy) return;
  busy = true;
  ready();
  clear();
  try {
    await parallax.removeGuard(el('account').value);
    configured = false;
    el('feedback').textContent = '已停用本机保存的令牌密钥；没有解绑Steam手机令牌或注销已保存登录态。';
    await status();
  } catch (e) {
    el('feedback').textContent = e.message;
  } finally {
    busy = false;
    ready();
  }
};

function tick() {
  if (!expires || !currentCode) return;
  const remaining = Math.max(0, expires - Math.floor(Date.now() / 1000));
  el('remaining').textContent = `剩余 ${remaining} 秒 · 本机时钟`;
  el('progress').value = remaining;
  if (remaining <= 5 && remaining > 0) {
    el('code').classList.add('expiring');
    el('progress').classList.add('expiring');
  } else {
    el('code').classList.remove('expiring');
    el('progress').classList.remove('expiring');
  }
  if (!remaining) {
    clear();
    el('remaining').textContent = '验证码已到期，请重新生成。';
  }
}

setInterval(tick, 1000);
document.addEventListener('visibilitychange', () => {
  if (document.hidden) clear();
});
window.addEventListener('parallax:pagechange', ({detail}) => {
  if (detail.id !== 'steam-guard') {
    clear();
    el('secret').value = '';
  }
});

ready();
refresh();
