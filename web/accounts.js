import { parallax } from './lib/api.js';

const el = id => document.getElementById(id);
const select = el('account-select');
const platform = el('session-platform');
const form = el('session-form');
let login = null;
let allAccounts = [];
let activeAccountId = null;

function renderLogin() {
  const pending = login?.status === 'pending';
  const btnStart = el('browser-start');
  const btnFinish = el('browser-finish');
  const btnCancel = el('browser-cancel');
  if (btnStart) btnStart.disabled = pending;
  if (btnFinish) btnFinish.disabled = !pending;
  if (btnCancel) btnCancel.disabled = !pending;
  if (login && el('browser-feedback')) {
    el('browser-feedback').textContent = `${login.platform.toUpperCase()} · ${login.message} · 绑定账户 ${login.account_id} · 截止 ${new Date(login.expires_at*1000).toLocaleTimeString()}`;
  }
}

async function loadLogin() {
  try {
    const {item} = await parallax.browserLogin();
    login = item;
    renderLogin();
  } catch (e) {}
}

const states = {
  unverified: '尚未验证',
  valid: '验证有效',
  invalid: '失效或身份不符',
  unknown: '无法确认',
  rate_limited: '限流冷却',
  verification_required: '需浏览器验证',
  missing: '本地凭据已停用或缺失'
};


const profileCache = new Map();
async function getAccountProfile(accId) {
  if (profileCache.has(accId)) return profileCache.get(accId);
  try {
    const prof = await parallax.accountProfile(accId);
    profileCache.set(accId, prof);
    return prof;
  } catch (_) {
    return null;
  }
}

function decodeEntities(str) {
  if (!str) return '';
  const doc = new DOMParser().parseFromString(str, 'text/html');
  return doc.body.textContent || str;
}

function renderAccountDetail(acc, session) {
  const avatarEl = el('detail-avatar');
  if (!acc) {
    el('detail-display-name').textContent = '请选择一个账户';
    el('detail-sub-name').textContent = '在左侧点击账户卡片，或点击「+ 添加账号」';
    el('detail-status-badge').textContent = '未选择';
    el('detail-status-badge').className = 'status-badge';
    el('meta-label').textContent = '—';
    el('meta-steam-id').textContent = '—';
    el('meta-session-status').textContent = '—';
    el('meta-checked-at').textContent = '—';
    if (avatarEl) avatarEl.innerHTML = '✦';
    return;
  }

  const cleanLabel = decodeEntities(acc.label);
  el('detail-display-name').textContent = cleanLabel;
  el('detail-sub-name').textContent = `Steam ID: ${acc.steam_id}`;
  el('meta-label').textContent = cleanLabel;
  el('meta-steam-id').textContent = acc.steam_id;

  // Pre-fill username in password form if empty
  const unameInput = el('steam-login-username');
  if (unameInput && !unameInput.value) {
    unameInput.value = cleanLabel;
  }

  const statusText = session ? (states[session.status] || session.status) : '未导入凭据';
  const isValid = session?.status === 'valid';
  el('detail-status-badge').textContent = statusText;
  el('detail-status-badge').className = `status-badge ${isValid ? 'online' : 'warn'}`;
  el('meta-session-status').textContent = statusText;
  el('meta-checked-at').textContent = session?.checked_at ? new Date(session.checked_at * 1000).toLocaleString() : '尚未检查';

  if (avatarEl) avatarEl.innerHTML = `${acc.label.charAt(0).toUpperCase()}`;

  // Async load true Steam avatar & persona name
  getAccountProfile(acc.id).then(prof => {
    if (!prof) return;
    if (prof.persona_name && el('detail-display-name')) {
      el('detail-display-name').textContent = `${prof.persona_name}`;
      el('detail-sub-name').textContent = `${acc.label} · SteamID: ${acc.steam_id}`;
    }
    if (prof.avatar_url && avatarEl) {
      avatarEl.innerHTML = `<img class="detail-avatar-img" src="${prof.avatar_url}" alt="Avatar">`;
    }
  });
}

async function sessions() {
  const accountId = select.value || activeAccountId;
  if (!accountId) {
    renderAccountDetail(null, null);
    return;
  }
  try {
    const {items} = await parallax.sessions(accountId);
    const steamSession = items.find(s => s.platform === 'steam') || items[0];
    const acc = allAccounts.find(a => String(a.id) === String(accountId));
    renderAccountDetail(acc, steamSession);

    // Also update legacy session-list if present
    if (el('session-list')) {
      el('session-list').replaceChildren();
      for (const s of items) {
        const row = document.createElement('p');
        row.textContent = `${s.platform.toUpperCase()} · ${states[s.status] || s.status} · ${s.message}${s.checked_at ? ' · 检查于 '+new Date(s.checked_at*1000).toLocaleString() : ''}`;
        el('session-list').append(row);
      }
    }
  } catch (e) {
    console.error('Failed to load sessions:', e);
  }
}

function renderAccountCards(items, selectedId) {
  const container = el('accounts-card-list');
  if (!container) return;
  container.replaceChildren();

  if (items.length === 0) {
    container.innerHTML = '<div class="accounts-empty-hint">暂无账号，点击上方「+ 添加账号」开始创建</div>';
    return;
  }

  items.forEach(acc => {
    const card = document.createElement('div');
    const isSelected = String(acc.id) === String(selectedId);
    const labelClean = decodeEntities(acc.label);
    card.className = `account-item-card ${isSelected ? 'active' : ''}`;
    card.dataset.id = acc.id;
    card.innerHTML = `
      <div class="acc-card-avatar" id="acc-card-avatar-${acc.id}">${labelClean.charAt(0).toUpperCase()}</div>
      <div class="acc-card-body">
        <div class="acc-card-name-row">
          <strong class="acc-card-name" id="acc-card-name-${acc.id}">${labelClean}</strong>
          ${isSelected ? '<span class="acc-current-tag">当前</span>' : ''}
        </div>
        <div class="acc-card-steamid">${acc.steam_id}</div>
      </div>
    `;

    // Async load avatar for this card
    getAccountProfile(acc.id).then(prof => {
      if (!prof) return;
      const avBox = document.getElementById(`acc-card-avatar-${acc.id}`);
      if (avBox && prof.avatar_url) {
        avBox.innerHTML = `<img class="acc-card-avatar-img" src="${prof.avatar_url}" alt="Avatar">`;
      }
      const nameBox = document.getElementById(`acc-card-name-${acc.id}`);
      if (nameBox && prof.persona_name) {
        nameBox.textContent = prof.persona_name;
      }
    });
    card.onclick = () => {
      selectAccount(acc.id);
    };
    container.appendChild(card);
  });
}

function selectAccount(id) {
  activeAccountId = id;
  select.value = id;
  renderAccountCards(allAccounts, id);
  action(sessions);
}

async function loadAccounts(preferred) {
  const {items} = await parallax.accounts();
  allAccounts = items;
  if (el('accounts-count')) el('accounts-count').textContent = items.length;

  select.replaceChildren();
  for (const a of items) {
    const o = document.createElement('option');
    o.value = a.id;
    o.textContent = `${a.label} · ${a.steam_id}`;
    select.append(o);
  }

  const chosenId = preferred || (items.some(a => String(a.id) === String(activeAccountId)) ? activeAccountId : (items[0]?.id || null));
  activeAccountId = chosenId;
  if (chosenId) select.value = chosenId;

  renderAccountCards(items, chosenId);
  await sessions();
}

async function action(fn) {
  const controls = [...document.querySelectorAll('#accounts-page button, #account-detail-card button')];
  controls.forEach(c => c.disabled = true);
  if (el('account-feedback')) el('account-feedback').textContent = '处理中…';
  try {
    const msg = await fn() || '已完成';
    if (el('account-feedback')) el('account-feedback').textContent = msg;
  } catch(e) {
    if (el('account-feedback')) el('account-feedback').textContent = e.message;
  } finally {
    controls.forEach(c => c.disabled = false);
    renderLogin();
  }
}

// Modal handling
function initModal() {
  const modal = el('account-modal-overlay');
  const btnShow = el('btn-show-add-account');
  const btnClose = el('btn-close-account-modal');
  const btnCancel = el('btn-cancel-account-modal');

  if (btnShow && modal) {
    btnShow.onclick = () => {
      modal.classList.remove('hidden');
      el('account-form')?.reset();
      modal.querySelector('input')?.focus();
    };
  }
  const hide = () => modal?.classList.add('hidden');
  if (btnClose) btnClose.onclick = hide;
  if (btnCancel) btnCancel.onclick = hide;
}

// Account form submit
el('account-form').onsubmit = e => {
  e.preventDefault();
  action(async () => {
    const values = Object.fromEntries(new FormData(e.target));
    const account = await parallax.createAccount(values);
    window.dispatchEvent(new Event('parallax:account-created'));
    e.target.reset();
    el('account-modal-overlay')?.classList.add('hidden');
    await loadAccounts(account.id);
    return '账户已成功添加';
  });
};

// Search filter
if (el('accounts-search')) {
  el('accounts-search').oninput = e => {
    const q = e.target.value.trim().toLowerCase();
    const filtered = allAccounts.filter(a => a.label.toLowerCase().includes(q) || a.steam_id.includes(q));
    renderAccountCards(filtered, activeAccountId);
  };
}

// Session import
if (form) {
  form.onsubmit = e => {
    e.preventDefault();
    action(async () => {
      const curId = select.value || activeAccountId;
      if (!curId) throw new Error('请先选择账户');
      const saved = await parallax.importSession(curId, platform.value, Object.fromEntries(new FormData(form)));
      if (form.elements.cookies) form.elements.cookies.value = '';
      await sessions();
      return saved.message;
    });
  };
}

if (el('session-verify')) {
  el('session-verify').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    const result = await parallax.verifySession(curId, platform.value);
    await sessions();
    return result.message;
  });
}

if (el('session-revoke')) {
  el('session-revoke').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    const result = await parallax.revokeSession(curId, platform.value);
    await sessions();
    return result.message;
  });
}

if (el('browser-start')) {
  el('browser-start').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    login = await parallax.startBrowserLogin(curId, platform.value);
    renderLogin();
    return login.message;
  });
}

if (el('browser-finish')) {
  el('browser-finish').onclick = () => action(async () => {
    if (!login) throw new Error('请先打开登录窗口');
    const result = await parallax.finishBrowserLogin(login.id);
    login = result;
    renderLogin();
    await sessions();
    return result.message;
  });
}

if (el('browser-cancel')) {
  el('browser-cancel').onclick = () => action(async () => {
    if (!login) return;
    login = await parallax.cancelBrowserLogin(login.id);
    renderLogin();
    return login.message;
  });
}

initModal();
loadAccounts().catch(e => {
  if (el('account-feedback')) el('account-feedback').textContent = e.message;
});
loadLogin().catch(e => {
  if (el('browser-feedback')) el('browser-feedback').textContent = e.message;
});



// Tab Switching for Credential Hub
document.querySelectorAll('.login-hub-tab-btn').forEach(btn => {
  btn.onclick = () => {
    document.querySelectorAll('.login-hub-tab-btn').forEach(b => {
      b.classList.remove('active');
      b.setAttribute('aria-selected', 'false');
    });
    document.querySelectorAll('.login-hub-panel').forEach(p => p.classList.remove('active'));
    btn.classList.add('active');
    btn.setAttribute('aria-selected', 'true');
    const target = document.getElementById(`tab-panel-${btn.dataset.tab}`);
    if (target) target.classList.add('active');
  };
});

// Standalone Browser Hub Trigger
const btnStartHub = el('browser-start-hub');
if (btnStartHub) {
  btnStartHub.onclick = () => {
    if (el('browser-start')) el('browser-start').click();
  };
}
const btnFinishHub = el('browser-finish-hub');
if (btnFinishHub) {
  btnFinishHub.onclick = () => {
    if (el('browser-finish')) el('browser-finish').click();
  };
}
const btnCancelHub = el('browser-cancel-hub');
if (btnCancelHub) {
  btnCancelHub.onclick = () => {
    if (el('browser-cancel')) el('browser-cancel').click();
  };
}

// Sync Browser Flow Buttons state
const originalRenderLogin = renderLogin;
renderLogin = function() {
  originalRenderLogin();
  const pending = login?.status === 'pending';
  if (btnStartHub) btnStartHub.disabled = pending;
  if (btnFinishHub) btnFinishHub.disabled = !pending;
  if (btnCancelHub) btnCancelHub.disabled = !pending;
  if (el('browser-feedback-hub') && login) {
    el('browser-feedback-hub').textContent = `${login.platform.toUpperCase()} · ${login.message} · 绑定账户 ${login.account_id} · 截止 ${new Date(login.expires_at*1000).toLocaleTimeString()}`;
  }
};


// Steam Password + Guard 2FA Login
const passwordForm = el('steam-password-form');
if (passwordForm) {
  passwordForm.onsubmit = async event => {
    event.preventDefault();
    const status = el('steam-password-status');
    const id = select.value || activeAccountId;
    if (!id) {
      if (status) status.textContent = '请先在左侧选择需要登录的账户。';
      return;
    }
    const submit = passwordForm.querySelector('button[type="submit"]');
    if (submit && submit.disabled) return;

    const usernameInput = passwordForm.elements.username;
    const passwordInput = passwordForm.elements.password;
    const username = (usernameInput?.value || '').trim();
    const password = passwordInput?.value || '';

    if (!username || !password) {
      if (status) status.textContent = '请输入 Steam 账户名与密码。';
      return;
    }

    // Clear password immediately after capturing into memory
    passwordInput.value = '';

    if (submit) {
      submit.disabled = true;
      submit.dataset.originalText = submit.innerHTML;
      submit.innerHTML = '<span>⏳</span> 正在登录并核验 Steam…';
    }
    if (status) status.textContent = '正在通过 Steam 协议握手并校验 Guard 令牌…';

    try {
      const result = await parallax.steamPasswordLogin(id, username, password);
      if (status) status.textContent = `✅ ${result.message || '登录成功，登录态已更新'}`;
      await sessions();
    } catch (error) {
      if (status) status.textContent = `❌ 登录失败：${error.message || '网络或密码错误'}`;
    } finally {
      if (submit) {
        submit.disabled = false;
        submit.innerHTML = submit.dataset.originalText || '<span>⚡</span> 验证并登录 Steam';
      }
    }
  };
}
