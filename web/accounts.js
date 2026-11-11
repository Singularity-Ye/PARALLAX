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
  const btnStartHub = el('browser-start-hub');
  if (btnStart) btnStart.disabled = pending;
  if (btnStartHub) btnStartHub.disabled = pending;
  if (btnFinish) btnFinish.disabled = !pending;
  if (btnCancel) btnCancel.disabled = !pending;
  if (login && el('browser-feedback')) {
    el('browser-feedback').textContent = `${login.platform.toUpperCase()} · ${login.message} · 绑定账户 ${login.account_id} · 截止 ${new Date(login.expires_at * 1000).toLocaleTimeString()}`;
  }
}

async function loadLogin() {
  try {
    const { item } = await parallax.browserLogin();
    login = item;
    renderLogin();
  } catch (_) {}
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
async function getAccountProfile(accId, forceRefresh = false) {
  if (!forceRefresh && profileCache.has(accId)) return profileCache.get(accId);
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
    if (el('meta-username')) el('meta-username').textContent = '—';
    if (el('meta-label')) el('meta-label').textContent = '—';
    if (el('meta-steam-id')) el('meta-steam-id').textContent = '—';
    if (el('meta-session-status')) el('meta-session-status').textContent = '—';
    if (el('meta-currency')) el('meta-currency').textContent = '—';
    if (el('meta-region')) el('meta-region').textContent = '—';
    if (el('meta-checked-at')) el('meta-checked-at').textContent = '—';
    if (avatarEl) avatarEl.innerHTML = '✦';
    return;
  }

  const cleanLabel = decodeEntities(acc.label);
  const displaySteamId = acc.steam_id === '00000000000000000' ? '待登录自动获取' : acc.steam_id;
  el('detail-display-name').textContent = cleanLabel;
  el('detail-sub-name').textContent = `Steam ID: ${displaySteamId}`;
  if (el('meta-label')) el('meta-label').textContent = cleanLabel;
  if (el('meta-steam-id')) el('meta-steam-id').textContent = displaySteamId;

  const statusText = session ? (states[session.status] || session.status) : '待验证登录';
  const isValid = session?.status === 'valid';
  el('detail-status-badge').textContent = isValid ? '当前在线' : statusText;
  el('detail-status-badge').className = `status-badge ${isValid ? 'online' : 'warn'}`;
  if (el('meta-session-status')) el('meta-session-status').textContent = statusText;
  if (el('meta-checked-at')) {
    el('meta-checked-at').textContent = session?.checked_at
      ? new Date(session.checked_at * 1000).toLocaleString()
      : '尚未检查';
  }

  if (avatarEl) avatarEl.innerHTML = `${cleanLabel.charAt(0).toUpperCase()}`;

  // Async load true Steam avatar, persona name, username, wallet balance, currency & region
  getAccountProfile(acc.id).then(prof => {
    if (!prof) return;
    const trueName = decodeEntities(prof.persona_name || cleanLabel);
    const uname = prof.username || cleanLabel;
    if (el('detail-display-name')) {
      el('detail-display-name').textContent = trueName;
      el('detail-sub-name').textContent = `${uname} · ${prof.steam_id === '00000000000000000' ? '待自动绑定 SteamID' : prof.steam_id}`;
    }
    if (el('meta-username')) el('meta-username').textContent = uname || '—';
    if (el('meta-label')) el('meta-label').textContent = trueName || '—';
    if (el('meta-steam-id') && prof.steam_id && prof.steam_id !== '00000000000000000') {
      el('meta-steam-id').textContent = prof.steam_id;
    }
    if (el('meta-currency')) {
      const c = prof.currency_code || '—';
      const bal = prof.wallet_balance_display ? ` · ${prof.wallet_balance_display}` : '';
      el('meta-currency').textContent = `${c}${bal}`;
    }
    if (el('meta-region')) {
      el('meta-region').textContent = prof.region_code || (session?.checked_at ? new Date(session.checked_at * 1000).toLocaleTimeString() : '—');
    }
    if (prof.avatar_url && avatarEl) {
      avatarEl.innerHTML = `<img class="detail-avatar-img" src="${prof.avatar_url}" alt="Avatar">`;
      if (el('meta-session-status')) {
        el('meta-session-status').textContent = `已获取头像 · ${statusText}`;
      }
    }
    const unameInput = el('steam-login-username');
    if (unameInput) {
      unameInput.value = uname;
    }
    const pwdBadge = el('saved-pwd-badge');
    if (pwdBadge) {
      pwdBadge.textContent = prof.has_saved_password
        ? '（已保存密码，留空可直接点验证免密登录）'
        : '（首次登录请填写密码，将自动本机加密保存）';
    }
  });
}

async function sessions(forceRefreshProfile = false) {
  const accountId = select.value || activeAccountId;
  if (!accountId) {
    renderAccountDetail(null, null);
    return;
  }
  if (forceRefreshProfile) {
    profileCache.delete(accountId);
  }
  try {
    const { items } = await parallax.sessions(accountId);
    const steamSession = items.find(s => s.platform === 'steam') || items[0];
    const acc = allAccounts.find(a => String(a.id) === String(accountId));
    renderAccountDetail(acc, steamSession);

    if (el('session-list')) {
      el('session-list').replaceChildren();
      for (const s of items) {
        const row = document.createElement('p');
        row.textContent = `${s.platform.toUpperCase()} · ${states[s.status] || s.status} · ${s.message}${s.checked_at ? ' · 检查于 ' + new Date(s.checked_at * 1000).toLocaleString() : ''}`;
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
    const sidClean = acc.steam_id === '00000000000000000' ? '待验证获取 SteamID' : acc.steam_id;
    card.className = `account-item-card ${isSelected ? 'active' : ''}`;
    card.dataset.id = acc.id;
    card.innerHTML = `
      <div class="acc-card-avatar" id="acc-card-avatar-${acc.id}">${labelClean.charAt(0).toUpperCase()}</div>
      <div class="acc-card-body">
        <div class="acc-card-name-row">
          <strong class="acc-card-name" id="acc-card-name-${acc.id}">${labelClean}</strong>
          ${isSelected ? '<span class="acc-current-tag">当前</span>' : ''}
        </div>
        <div class="acc-card-steamid" id="acc-card-sub-${acc.id}">${sidClean}</div>
      </div>
    `;

    getAccountProfile(acc.id).then(prof => {
      if (!prof) return;
      const avBox = document.getElementById(`acc-card-avatar-${acc.id}`);
      if (avBox && prof.avatar_url) {
        avBox.innerHTML = `<img class="acc-card-avatar-img" src="${prof.avatar_url}" alt="Avatar">`;
      }
      const nameBox = document.getElementById(`acc-card-name-${acc.id}`);
      if (nameBox && prof.persona_name) {
        nameBox.textContent = decodeEntities(prof.persona_name);
      }
      const subBox = document.getElementById(`acc-card-sub-${acc.id}`);
      if (subBox) {
        const u = prof.username ? `${prof.username} · ` : '';
        const sid = (prof.steam_id && prof.steam_id !== '00000000000000000') ? prof.steam_id : sidClean;
        subBox.textContent = `${u}${sid}`;
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

async function loadAccounts(preferred, forceRefreshProfile = false) {
  const { items } = await parallax.accounts();
  allAccounts = items;
  if (el('accounts-count')) el('accounts-count').textContent = items.length;

  select.replaceChildren();
  for (const a of items) {
    const o = document.createElement('option');
    o.value = a.id;
    o.textContent = `${decodeEntities(a.label)} · ${a.steam_id}`;
    select.append(o);
  }

  const chosenId = preferred || (items.some(a => String(a.id) === String(activeAccountId)) ? activeAccountId : (items[0]?.id || null));
  activeAccountId = chosenId;
  if (chosenId) select.value = chosenId;

  if (forceRefreshProfile && chosenId) {
    profileCache.delete(chosenId);
  }
  renderAccountCards(items, chosenId);
  await sessions(forceRefreshProfile);
}

async function action(fn) {
  const controls = [...document.querySelectorAll('#accounts-page button, #account-detail-card button')];
  controls.forEach(c => c.disabled = true);
  if (el('account-feedback')) el('account-feedback').textContent = '处理中…';
  try {
    const msg = await fn() || '已完成';
    if (el('account-feedback')) el('account-feedback').textContent = msg;
  } catch (e) {
    if (el('account-feedback')) el('account-feedback').textContent = e.message;
  } finally {
    controls.forEach(c => c.disabled = false);
    renderLogin();
  }
}

// Modal handling (Add & Edit)
function initModal() {
  const modal = el('account-modal-overlay');
  const btnShow = el('btn-show-add-account');
  const btnEdit = el('btn-account-edit');
  const btnClose = el('btn-close-account-modal');
  const btnCancel = el('btn-cancel-account-modal');

  if (btnShow && modal) {
    btnShow.onclick = () => {
      el('account-form')?.reset();
      if (el('account-form-edit-id')) el('account-form-edit-id').value = '';
      if (el('account-modal-title')) el('account-modal-title').textContent = '添加 Steam 账号';
      if (el('btn-submit-account-modal')) el('btn-submit-account-modal').textContent = '保存并立即验证';
      modal.classList.remove('hidden');
      el('modal-input-username')?.focus();
    };
  }
  if (btnEdit && modal) {
    btnEdit.onclick = async () => {
      const curId = select.value || activeAccountId;
      const acc = allAccounts.find(a => String(a.id) === String(curId));
      if (!acc) return;
      const prof = await getAccountProfile(acc.id);
      el('account-form')?.reset();
      if (el('account-form-edit-id')) el('account-form-edit-id').value = acc.id;
      if (el('account-modal-title')) el('account-modal-title').textContent = '编辑 Steam 账号';
      if (el('btn-submit-account-modal')) el('btn-submit-account-modal').textContent = '保存修改';
      if (el('modal-input-username')) el('modal-input-username').value = prof?.username || '';
      if (el('modal-input-label')) el('modal-input-label').value = decodeEntities(acc.label);
      if (el('modal-input-steamid')) {
        el('modal-input-steamid').value = acc.steam_id === '00000000000000000' ? '' : acc.steam_id;
      }
      modal.classList.remove('hidden');
      el('modal-input-username')?.focus();
    };
  }
  const hide = () => modal?.classList.add('hidden');
  if (btnClose) btnClose.onclick = hide;
  if (btnCancel) btnCancel.onclick = hide;
}

// Account form submit (Add or Edit + Auto-Login)
el('account-form').onsubmit = e => {
  e.preventDefault();
  action(async () => {
    const raw = Object.fromEntries(new FormData(e.target));
    const editId = (raw.edit_id || '').trim();
    const username = (raw.username || '').trim();
    const password = raw.password || '';
    const sharedSecret = (raw.shared_secret || '').trim();
    const label = (raw.label || '').trim();
    const steamId = (raw.steam_id || '').trim();

    let targetId = editId;
    if (editId) {
      await parallax.updateAccount(editId, {
        label: label || undefined,
        steam_id: steamId || undefined,
        username: username || undefined,
        password: password || undefined,
      });
    } else {
      const created = await parallax.createAccount({
        label,
        steam_id: steamId,
        username: username || undefined,
        password: password || undefined,
      });
      targetId = created.id;
    }

    if (sharedSecret && targetId) {
      await parallax.saveGuard(targetId, sharedSecret);
    }

    e.target.reset();
    el('account-modal-overlay')?.classList.add('hidden');

    // If username & password were provided, immediately run automatic Steam login & profile sync!
    if (targetId && username && password) {
      if (el('steam-password-status')) {
        el('steam-password-status').textContent = '正在通过 Steam 原生协议自动登录并同步头像与钱包币种…';
      }
      try {
        const res = await parallax.steamPasswordLogin(targetId, username, password);
        window.dispatchEvent(new Event('parallax:account-created'));
        await loadAccounts(targetId, true);
        if (el('steam-password-status')) {
          el('steam-password-status').textContent = `${res.message}`;
        }
        return `账号已保存并完成登录：${res.message}`;
      } catch (loginErr) {
        window.dispatchEvent(new Event('parallax:account-created'));
        await loadAccounts(targetId, true);
        if (el('steam-password-status')) {
          el('steam-password-status').textContent = `账号已保存，自动登录待确认：${loginErr.message}`;
        }
        return `账号已保存，但自动登录未完成：${loginErr.message}`;
      }
    }

    window.dispatchEvent(new Event('parallax:account-created'));
    await loadAccounts(targetId, true);
    return editId ? '账户信息已更新' : '账户已成功添加';
  });
};

// Quick Verify / 1-Click Auto-Relogin button on Account Detail Header
if (el('btn-account-quick-verify')) {
  el('btn-account-quick-verify').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    const status = el('steam-password-status');
    const uname = (el('steam-login-username')?.value || '').trim();
    const pwd = el('steam-login-password')?.value || '';
    const guardCode = (el('steam-login-guard-code')?.value || '').trim();
    if (el('steam-login-password')) el('steam-login-password').value = '';

    if (status) status.textContent = '正在核验会话或执行免密自动登录…';

    // 1. If user typed a password or has saved password, try password/auto-relogin or session verify
    try {
      const prof = await getAccountProfile(curId);
      if (pwd || prof?.has_saved_password) {
        const res = await parallax.steamPasswordLogin(curId, uname, pwd, guardCode ? { guard_code: guardCode } : {});
        await loadAccounts(curId, true);
        if (status) status.textContent = `${res.message}`;
        return `${res.message}`;
      }
      const res = await parallax.verifySession(curId, 'steam');
      await loadAccounts(curId, true);
      if (status) status.textContent = `${res.message}`;
      return `${res.message}`;
    } catch (err) {
      // Fallback: if verifySession failed and saved password exists, try steamPasswordLogin
      if (status) status.textContent = `失败：${err.message}`;
      throw err;
    }
  });
}

// Delete account button
if (el('btn-account-delete')) {
  el('btn-account-delete').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    await parallax.deleteAccount(curId);
    profileCache.delete(curId);
    window.dispatchEvent(new Event('parallax:account-created'));
    await loadAccounts(null, true);
    return '账户及本地加密凭据已删除';
  });
}

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
      await sessions(true);
      return saved.message;
    });
  };
}

if (el('session-verify')) {
  el('session-verify').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    const result = await parallax.verifySession(curId, platform.value);
    await sessions(true);
    return result.message;
  });
}

if (el('session-revoke')) {
  el('session-revoke').onclick = () => action(async () => {
    const curId = select.value || activeAccountId;
    if (!curId) throw new Error('请先选择账户');
    const result = await parallax.revokeSession(curId, platform.value);
    await sessions(true);
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
    await sessions(true);
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

// Steam Password + Guard 2FA Login Form (Supports saved password when password field is left blank!)
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
    const guardInput = passwordForm.elements.guard_code;
    const username = (usernameInput?.value || '').trim();
    const password = passwordInput?.value || '';
    const guardCode = (guardInput?.value || '').trim();

    // Clear password & one-time guard code immediately after capturing into memory
    if (passwordInput) passwordInput.value = '';
    if (guardInput) guardInput.value = '';

    if (submit) {
      submit.disabled = true;
      submit.dataset.originalText = submit.innerHTML;
      submit.innerHTML = '正在登录并同步 Steam 资料…';
    }
    if (status) status.textContent = '正在通过 Steam 原生协议握手并校验 Guard 令牌（如使用手机确认请在 App 中点击批准）…';

    try {
      const result = await parallax.steamPasswordLogin(
        id,
        username,
        password,
        guardCode ? { guard_code: guardCode } : {}
      );
      if (status) status.textContent = `${result.message || '登录成功，登录态与个人资料已更新'}`;
      await loadAccounts(id, true);
    } catch (error) {
      if (status) status.textContent = `失败：${error.message || '网络或密码错误'}`;
    } finally {
      if (submit) {
        submit.disabled = false;
        submit.innerHTML = submit.dataset.originalText || '验证并登录 Steam';
      }
    }
  };
}

initModal();
loadAccounts().catch(e => {
  if (el('account-feedback')) el('account-feedback').textContent = e.message;
});
loadLogin().catch(e => {
  if (el('browser-feedback')) el('browser-feedback').textContent = e.message;
});
