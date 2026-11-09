import { parallax } from './lib/api.js';

const el = id => document.getElementById(id);

let currentNetworkConfig = {
  enabled: false,
  nodes: [],
  routes: { steam: 'direct', buff: 'direct', steamdt: 'direct', fx: 'direct' }
};
let isPendingRestart = false;

const platforms = ['steam', 'buff', 'steamdt', 'fx'];

async function loadSettings() {
  const feedback = el('settings-save-feedback');
  if (feedback) feedback.textContent = '';

  try {
    const res = await parallax.networkSettings();
    if (res && res.config) {
      currentNetworkConfig = res.config;
      isPendingRestart = Boolean(res.pending_restart);
    }
  } catch (err) {
    console.warn('Failed to load settings from /api/settings/network:', err);
  }

  renderSettings();
}

function renderSettings() {
  // 1. Pending Restart Banner
  const banner = el('settings-restart-banner');
  if (banner) {
    banner.style.display = isPendingRestart ? 'flex' : 'none';
  }

  // 2. Network Enabled Checkbox
  const chkEnabled = el('setting-network-enabled');
  if (chkEnabled) {
    chkEnabled.checked = Boolean(currentNetworkConfig.enabled);
  }

  // 3. Populate 4 Route Dropdowns
  const nodes = currentNetworkConfig.nodes || [];
  const routes = currentNetworkConfig.routes || {};

  platforms.forEach(plat => {
    const select = el(`route-select-${plat}`);
    const badge = el(`badge-route-${plat}`);
    if (!select) return;

    select.replaceChildren();

    // Option 1: Direct
    const optDirect = document.createElement('option');
    optDirect.value = 'direct';
    optDirect.textContent = '直连 (direct) · 不走代理';
    select.appendChild(optDirect);

    // Option nodes
    nodes.forEach(n => {
      const opt = document.createElement('option');
      opt.value = n.id;
      opt.textContent = `节点 [${n.id}] · ${n.url}`;
      select.appendChild(opt);
    });

    const chosen = routes[plat] || 'direct';
    select.value = chosen;

    // Update Head Badge
    if (badge) {
      const isDir = chosen === 'direct';
      badge.textContent = isDir ? 'DIRECT' : `NODE: ${chosen}`;
      badge.className = `badge ${isDir ? 'muted' : 'ok'}`;
    }

    select.onchange = () => {
      const newVal = select.value;
      if (badge) {
        const isDir = newVal === 'direct';
        badge.textContent = isDir ? 'DIRECT' : `NODE: ${newVal}`;
        badge.className = `badge ${isDir ? 'muted' : 'ok'}`;
      }
    };
  });
}

async function saveSettings() {
  const btnSave = el('btn-save-settings');
  const feedback = el('settings-save-feedback');
  if (btnSave) btnSave.disabled = true;
  if (feedback) feedback.textContent = '正在保存平台网络出口策略…';

  try {
    const chkEnabled = el('setting-network-enabled');
    const newEnabled = chkEnabled ? chkEnabled.checked : currentNetworkConfig.enabled;

    const newRoutes = { ...currentNetworkConfig.routes };
    platforms.forEach(plat => {
      const select = el(`route-select-${plat}`);
      if (select) {
        newRoutes[plat] = select.value || 'direct';
      }
    });

    const payload = {
      enabled: newEnabled,
      nodes: currentNetworkConfig.nodes || [],
      routes: newRoutes
    };

    const res = await parallax.saveNetworkSettings(payload);
    currentNetworkConfig = payload;
    isPendingRestart = Boolean(res.pending_restart);
    renderSettings();

    if (feedback) {
      feedback.textContent = isPendingRestart
        ? '✅ 网络出口策略已成功保存！配置将在本地 38480 实例重启后生效。'
        : '✅ 网络出口策略已成功保存！';
    }
  } catch (err) {
    if (feedback) feedback.textContent = `❌ 保存失败：${err.message}`;
  } finally {
    if (btnSave) btnSave.disabled = false;
  }
}


let currentNotifyConfig = {
  enabled: false,
  pushplus_token: '',
  notify_on_sold: true,
  notify_on_auth_expired: true,
  notify_on_large_price_drop: false,
  price_drop_threshold_pct: 20.0
};

async function loadNotifySettings() {
  try {
    const res = await parallax.notifySettings();
    if (res) {
      currentNotifyConfig = res;
      renderNotifySettings();
    }
  } catch (err) {
    console.warn('Failed to load notify settings:', err);
  }
}

function renderNotifySettings() {
  const chk = el('notify-enabled');
  if (chk) chk.checked = Boolean(currentNotifyConfig.enabled);

  const tokenInput = el('notify-token');
  if (tokenInput) {
    tokenInput.value = currentNotifyConfig.pushplus_token || '';
  }

  const chkSold = el('notify-on-sold');
  if (chkSold) chkSold.checked = Boolean(currentNotifyConfig.notify_on_sold);

  const chkAuth = el('notify-on-auth-expired');
  if (chkAuth) chkAuth.checked = Boolean(currentNotifyConfig.notify_on_auth_expired);

  const chkDrop = el('notify-on-price-drop');
  if (chkDrop) chkDrop.checked = Boolean(currentNotifyConfig.notify_on_large_price_drop);
}

async function saveNotifySettings() {
  const btn = el('btn-save-notify');
  const feedback = el('notify-save-feedback');
  if (btn) btn.disabled = true;
  if (feedback) feedback.textContent = '正在保存微信通知设置…';

  try {
    const chk = el('notify-enabled');
    const tokenInput = el('notify-token');
    const chkSold = el('notify-on-sold');
    const chkAuth = el('notify-on-auth-expired');
    const chkDrop = el('notify-on-price-drop');

    const payload = {
      enabled: chk ? chk.checked : false,
      pushplus_token: tokenInput ? tokenInput.value.trim() : '',
      notify_on_sold: chkSold ? chkSold.checked : true,
      notify_on_auth_expired: chkAuth ? chkAuth.checked : true,
      notify_on_large_price_drop: chkDrop ? chkDrop.checked : false,
      price_drop_threshold_pct: 20.0
    };

    const res = await parallax.saveNotifySettings(payload);
    if (res) {
      currentNotifyConfig = res;
      renderNotifySettings();
    }
    if (feedback) feedback.textContent = '✅ 微信推送通知配置已成功保存并立即生效！';
  } catch (err) {
    if (feedback) feedback.textContent = `❌ 保存失败：${err.message}`;
  } finally {
    if (btn) btn.disabled = false;
  }
}

async function testNotify() {
  const btn = el('btn-test-notify');
  const feedback = el('notify-save-feedback');
  if (btn) btn.disabled = true;
  if (feedback) feedback.textContent = '正在发送 PushPlus 微信测试通知…';

  try {
    const res = await parallax.testNotify();
    if (res && res.success) {
      if (feedback) feedback.textContent = `✅ ${res.message || '测试通知已成功投递！请检查微信客户端通知。'}`;
    } else {
      if (feedback) feedback.textContent = `⚠️ 发送反馈：${res ? res.message : '未明确'}`;
    }
  } catch (err) {
    if (feedback) feedback.textContent = `❌ 测试发送失败：${err.message}`;
  } finally {
    if (btn) btn.disabled = false;
  }
}

function initSettings() {
  const btnSave = el('btn-save-settings');
  if (btnSave) btnSave.onclick = () => saveSettings();

  const btnSaveNotify = el('btn-save-notify');
  if (btnSaveNotify) btnSaveNotify.onclick = () => saveNotifySettings();

  const btnTestNotify = el('btn-test-notify');
  if (btnTestNotify) btnTestNotify.onclick = () => testNotify();

  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'settings') {
      loadSettings();
      loadNotifySettings();
    }
  });
  window.addEventListener('parallax:network-settings-updated', event => {
    if (event.detail) {
      currentNetworkConfig = event.detail;
      renderSettings();
    }
  });

  if (location.hash === '#settings') {
    loadSettings();
    loadNotifySettings();
  }
}

initSettings();
loadSettings().catch(console.error);
loadNotifySettings().catch(console.error);
