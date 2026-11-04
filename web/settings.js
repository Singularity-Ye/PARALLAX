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

function initSettings() {
  const btnSave = el('btn-save-settings');
  if (btnSave) btnSave.onclick = () => saveSettings();

  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'settings') loadSettings();
  });
  window.addEventListener('parallax:network-settings-updated', event => {
    if (event.detail) {
      currentNetworkConfig = event.detail;
      renderSettings();
    }
  });

  if (location.hash === '#settings') loadSettings();
}

initSettings();
loadSettings().catch(console.error);
