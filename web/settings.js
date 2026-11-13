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
        ? '网络出口策略已成功保存！配置将在本地 38480 实例重启后生效。'
        : '网络出口策略已成功保存！';
    }
  } catch (err) {
    if (feedback) feedback.textContent = `保存失败：${err.message}`;
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
    if (feedback) feedback.textContent = '微信推送通知配置已成功保存并立即生效！';
  } catch (err) {
    if (feedback) feedback.textContent = `保存失败：${err.message}`;
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
      if (feedback) feedback.textContent = `${res.message || '测试通知已成功投递！请检查微信客户端通知。'}`;
    } else {
      if (feedback) feedback.textContent = `发送反馈：${res ? res.message : '未明确'}`;
    }
  } catch (err) {
    if (feedback) feedback.textContent = `测试发送失败：${err.message}`;
  } finally {
    if (btn) btn.disabled = false;
  }
}

function formatBytes(bytes) {
  const n = Number(bytes || 0);
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / (1024 * 1024)).toFixed(2)} MB`;
  return `${(n / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function renderMaintenanceStats(stats) {
  if (!stats) return;
  const icons = stats.icons || {};
  const logs = stats.logs || {};
  if (el('maint-icon-summary')) {
    el('maint-icon-summary').textContent = `已缓存 ${icons.cached_files ?? 0} 个 · ${formatBytes(icons.cached_bytes)} / 上限 ${formatBytes(icons.max_bytes)}`;
  }
  if (el('maint-icon-sub')) {
    el('maint-icon-sub').textContent = `已索引饰品图标地址 ${icons.indexed_count ?? 0} 条 · 离线时直接读取本地缓存`;
  }
  if (el('maint-log-summary')) {
    el('maint-log-summary').textContent = `扫描流水 ${logs.scans_total ?? 0} 条（可清理 ${logs.scans_cleanable ?? 0} 条） · 过期快照 ${logs.snapshots_cleanable ?? 0} 条`;
  }
  if (el('maint-log-sub')) {
    const dbTotal = Number(logs.db_bytes || 0) + Number(logs.wal_bytes || 0);
    el('maint-log-sub').textContent = `数据库主体 ${formatBytes(logs.db_bytes)} + WAL 日志 ${formatBytes(logs.wal_bytes)}（合计 ${formatBytes(dbTotal)}）`;
  }
}

async function loadMaintenanceStats() {
  try {
    const stats = await parallax.maintenanceStats();
    renderMaintenanceStats(stats);
  } catch (err) {
    if (el('maint-feedback')) {
      el('maint-feedback').textContent = `统计加载失败：${err.message}`;
    }
  }
}

async function runMaintenanceCleanup({ clear_icons = false, clear_logs = false }) {
  const btnIds = ['btn-maint-clear-icons', 'btn-maint-clear-logs', 'btn-maint-clear-all', 'btn-maint-refresh'];
  btnIds.forEach(id => { if (el(id)) el(id).disabled = true; });
  const feedback = el('maint-feedback');
  if (feedback) feedback.textContent = '正在清理并整理本地存储…';
  try {
    const res = await parallax.cleanupMaintenance({ clear_icons, clear_logs, keep_scans: 1 });
    if (res?.stats) renderMaintenanceStats(res.stats);
    const parts = [];
    if (clear_icons) parts.push(`已清理图标文件 ${res.cleared_icon_files ?? 0} 个（释放 ${formatBytes(res.freed_icon_bytes)}）`);
    if (clear_logs) parts.push(`已删除过期扫描 ${res.deleted_scans ?? 0} 条、冗余快照 ${res.deleted_snapshots ?? 0} 条（回收库空间 ${formatBytes(res.reclaimed_db_bytes)}）`);
    if (feedback) feedback.textContent = `${parts.join('；')}。`;
    window.dispatchEvent(new CustomEvent('parallax:maintenance-cleaned', { detail: res }));
  } catch (err) {
    if (feedback) feedback.textContent = `清理失败：${err.message}`;
  } finally {
    btnIds.forEach(id => { if (el(id)) el(id).disabled = false; });
  }
}

function initSettings() {
  const btnSave = el('btn-save-settings');
  if (btnSave) btnSave.onclick = () => saveSettings();

  const btnSaveNotify = el('btn-save-notify');
  if (btnSaveNotify) btnSaveNotify.onclick = () => saveNotifySettings();

  const btnTestNotify = el('btn-test-notify');
  if (btnTestNotify) btnTestNotify.onclick = () => testNotify();

  if (el('btn-maint-clear-icons')) {
    el('btn-maint-clear-icons').onclick = () => runMaintenanceCleanup({ clear_icons: true });
  }
  if (el('btn-maint-clear-logs')) {
    el('btn-maint-clear-logs').onclick = () => runMaintenanceCleanup({ clear_logs: true });
  }
  if (el('btn-maint-clear-all')) {
    el('btn-maint-clear-all').onclick = () => runMaintenanceCleanup({ clear_icons: true, clear_logs: true });
  }
  if (el('btn-maint-refresh')) {
    el('btn-maint-refresh').onclick = () => loadMaintenanceStats();
  }

  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'settings') {
      loadSettings();
      loadNotifySettings();
      loadMaintenanceStats();
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
    loadMaintenanceStats();
  }
}

initSettings();
loadSettings().catch(console.error);
loadNotifySettings().catch(console.error);
loadMaintenanceStats().catch(() => {});
