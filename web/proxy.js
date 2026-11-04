import { parallax } from './lib/api.js';
import { node } from './lib/pricing-view.js';

const el = id => document.getElementById(id);

// Local state for proxy configuration draft & active status
let savedConfig = {
  enabled: false,
  nodes: [],
  routes: { steam: 'direct', buff: 'direct', steamdt: 'direct', fx: 'direct' }
};
let pendingRestart = false;
let activeStatus = null; // result of proxyStatus()

/**
 * Load both active runtime status and saved network config
 */
async function loadProxyData() {
  const feedback = el('proxy-save-feedback');
  if (feedback) feedback.textContent = '';

  try {
    // 1. Fetch saved config (/api/settings/network)
    const settingsRes = await parallax.networkSettings();
    if (settingsRes && settingsRes.config) {
      savedConfig = settingsRes.config;
      pendingRestart = Boolean(settingsRes.pending_restart);
    }
  } catch (err) {
    console.warn('Failed to fetch /api/settings/network, will use fallback:', err);
  }

  try {
    // 2. Fetch active runtime status (/api/network/proxies)
    activeStatus = await parallax.proxyStatus();
  } catch (err) {
    console.error('Failed to fetch /api/network/proxies:', err);
  }

  renderAll();
}

/**
 * Render all sections of the proxy page
 */
function renderAll() {
  renderRestartBanner();
  renderActiveRuntime();
  renderSavedNodesTable();
}

/**
 * Render Pending Restart Alert Banner
 */
function renderRestartBanner() {
  const banner = el('proxy-restart-banner');
  if (!banner) return;
  if (pendingRestart) {
    banner.style.display = 'flex';
  } else {
    banner.style.display = 'none';
  }
}

/**
 * Render Section 1: Active Runtime Routes (/api/network/proxies)
 */
function renderActiveRuntime() {
  const summaryBox = el('proxy-active-summary');
  const gridBox = el('proxy-active-routes-grid');
  if (!summaryBox || !gridBox) return;

  summaryBox.replaceChildren();
  gridBox.replaceChildren();

  if (!activeStatus) {
    summaryBox.innerHTML = '<div class="notice">暂未获取到当前进程网络状态。</div>';
    return;
  }

  const isExplicit = activeStatus.mode === 'explicit_routes';
  const summary = node('div');
  summary.className = 'proxy-summary';

  const summaryLeft = node('div');
  const modeBadge = node('span', `当前模式：${isExplicit ? '显式平台路由' : '沿用系统环境代理'}`);
  modeBadge.className = `badge ${isExplicit ? 'ok' : 'muted'}`;
  const desc = node('p', `生效出口节点 ${activeStatus.nodes.length} 个 · 进程固定出口（不自动轮换） · 修改配置后需重启 38480 实例。`);
  desc.className = 'hint';
  desc.style.margin = '4px 0 0';
  summaryLeft.append(modeBadge, desc);

  const summaryRight = node('div');
  summaryRight.className = 'proxy-meta';
  summaryRight.textContent = isExplicit ? '未单独指定的平台默认直连' : '所有网络请求沿用本地系统全局代理';
  summary.append(summaryLeft, summaryRight);
  summaryBox.append(summary);

  // Platform Cards
  const routeEntries = Object.entries(activeStatus.routes || {});
  if (!routeEntries.length) {
    const notice = node('div');
    notice.className = 'notice';
    notice.textContent = isExplicit
      ? '当前未定义显式平台映射路由。所有请求均以直连方式访问目标平台。'
      : '当前运行于系统环境代理模式，未指定平台专属出口。';
    gridBox.append(notice);
  } else {
    for (const [platform, target] of routeEntries) {
      const card = node('div');
      card.className = 'proxy-card';

      const head = node('div');
      head.className = 'proxy-card-head';
      head.append(node('h3', platform.toUpperCase()));
      const isDirect = target === 'direct';
      const targetBadge = node('span', isDirect ? '直连 (Direct)' : `节点 ${target}`);
      targetBadge.className = `badge ${isDirect ? 'muted' : 'ok'}`;
      head.append(targetBadge);

      const meta = node('div');
      meta.className = 'proxy-meta';
      meta.textContent = `该平台实际出站通道：${target}`;

      card.append(head, meta);

      // Probe button for active non-direct nodes
      if (!isDirect && activeStatus.nodes.includes(target)) {
        const probeBtn = node('button', '测试此节点连通');
        probeBtn.className = 'btn btn-sm btn-secondary';
        probeBtn.style.alignSelf = 'flex-start';
        probeBtn.style.marginTop = '6px';
        const resultBox = node('div');
        resultBox.style.fontSize = '11px';
        resultBox.style.marginTop = '4px';

        probeBtn.onclick = async () => {
          probeBtn.disabled = true;
          resultBox.textContent = '正在探测中…';
          try {
            const probeRes = await parallax.probeProxy(target);
            resultBox.textContent = `✅ 连通耗时 ${probeRes.latency_ms}ms · IP/出口: ${probeRes.message || 'OK'}`;
            resultBox.style.color = '#16a34a';
          } catch (e) {
            resultBox.textContent = `❌ 探测失败: ${e.message}`;
            resultBox.style.color = '#dc2626';
          } finally {
            probeBtn.disabled = false;
          }
        };
        card.append(probeBtn, resultBox);
      }

      gridBox.append(card);
    }
  }
}

/**
 * Render Section 2: Saved Node Manager Table (/api/settings/network)
 */
function renderSavedNodesTable() {
  const tbody = el('proxy-node-tbody');
  if (!tbody) return;

  tbody.replaceChildren();

  const nodes = savedConfig.nodes || [];
  if (nodes.length === 0) {
    const tr = node('tr');
    tr.className = 'empty-row';
    const td = node('td', '当前 proxy.json 尚未配置任何代理节点。点击上方「+ 添加节点」或「导入配置」添加。');
    td.colSpan = 6;
    td.className = 'empty-cell';
    tr.append(td);
    tbody.append(tr);
    return;
  }

  // Find which platforms reference each node
  const activeRoutes = savedConfig.routes || {};
  const routeReferences = {};
  for (const [plat, target] of Object.entries(activeRoutes)) {
    if (target !== 'direct') {
      if (!routeReferences[target]) routeReferences[target] = [];
      routeReferences[target].push(plat.toUpperCase());
    }
  }

  nodes.forEach(item => {
    const tr = node('tr');

    // 1. Node ID
    const tdId = node('td');
    const idTag = node('span', item.id);
    idTag.className = 'scan-id-tag';
    tdId.append(idTag);

    // 2. Node URL
    const tdUrl = node('td', item.url);
    tdUrl.className = 'col-mono';

    // 3. Protocol
    const isHttps = item.url.toLowerCase().startsWith('https:');
    const tdProto = node('td');
    const protoBadge = node('span', isHttps ? 'HTTPS' : 'HTTP');
    protoBadge.className = 'badge muted';
    tdProto.append(protoBadge);

    // 4. Referenced Platforms
    const tdRefs = node('td');
    const refs = routeReferences[item.id];
    if (refs && refs.length > 0) {
      refs.forEach(r => {
        const b = node('span', r);
        b.className = 'badge ok';
        b.style.marginRight = '4px';
        tdRefs.append(b);
      });
    } else {
      const noneSpan = node('span', '未引用');
      noneSpan.style.color = 'var(--muted)';
      noneSpan.style.fontSize = '11px';
      tdRefs.append(noneSpan);
    }

    // 5. Runtime Status & Probe
    const tdStatus = node('td');
    const isActiveInProcess = activeStatus?.nodes?.includes(item.id);
    if (isActiveInProcess) {
      const probeBtn = node('button', '探测');
      probeBtn.className = 'btn btn-sm btn-secondary';
      probeBtn.style.fontSize = '10px';
      probeBtn.style.padding = '2px 8px';
      const statusSpan = node('span', ' 生效中');
      statusSpan.style.fontSize = '10.5px';
      statusSpan.style.color = '#16a34a';
      probeBtn.onclick = async () => {
        probeBtn.disabled = true;
        statusSpan.textContent = ' 探测中…';
        try {
          const res = await parallax.probeProxy(item.id);
          statusSpan.textContent = ` ${res.latency_ms}ms`;
          statusSpan.style.color = '#16a34a';
        } catch (e) {
          statusSpan.textContent = ` 失败`;
          statusSpan.style.color = '#dc2626';
        } finally {
          probeBtn.disabled = false;
        }
      };
      tdStatus.append(probeBtn, statusSpan);
    } else {
      const unappliedSpan = node('span', pendingRestart ? '待重启加载' : '未加载');
      unappliedSpan.className = 'badge warn';
      tdStatus.append(unappliedSpan);
    }

    // 6. Actions (Delete)
    const tdAction = node('td');
    tdAction.style.textAlign = 'right';
    const delBtn = node('button', '删除');
    delBtn.className = 'btn btn-sm btn-danger-outline';
    delBtn.style.fontSize = '10.5px';
    delBtn.style.padding = '2px 8px';
    delBtn.onclick = () => {
      deleteNode(item.id);
    };
    tdAction.append(delBtn);

    tr.append(tdId, tdUrl, tdProto, tdRefs, tdStatus, tdAction);
    tbody.append(tr);
  });
}

/**
 * Delete a node from draft
 */
function deleteNode(id) {
  savedConfig.nodes = (savedConfig.nodes || []).filter(n => n.id !== id);
  // Reset any route pointing to this deleted node to direct
  if (savedConfig.routes) {
    for (const [k, v] of Object.entries(savedConfig.routes)) {
      if (v === id) savedConfig.routes[k] = 'direct';
    }
  }
  renderSavedNodesTable();
  const feedback = el('proxy-save-feedback');
  if (feedback) feedback.textContent = `已从草稿移除节点 ${id}，请点击「保存配置并写入」完成提交。`;
}

/**
 * Add a new node to draft
 */
function addNode(id, url) {
  const errBox = el('node-add-error');
  if (errBox) errBox.textContent = '';

  id = id.trim();
  url = url.trim();

  if (!id) {
    if (errBox) errBox.textContent = '节点标识 (ID) 不能为空。';
    return false;
  }
  if (!/^https?:\/\//i.test(url)) {
    if (errBox) errBox.textContent = '节点 URL 必须为 http:// 或 https:// 开头。';
    return false;
  }
  // Validate no credentials in URL
  try {
    const parsed = new URL(url);
    if (parsed.username || parsed.password) {
      if (errBox) errBox.textContent = '安全限制：只支持无密码凭据的代理 URL。';
      return false;
    }
  } catch (_) {
    if (errBox) errBox.textContent = '节点 URL 格式不合法。';
    return false;
  }

  if ((savedConfig.nodes || []).length >= 20) {
    if (errBox) errBox.textContent = '节点上限为 20 个，无法继续添加。';
    return false;
  }
  if ((savedConfig.nodes || []).some(n => n.id === id)) {
    if (errBox) errBox.textContent = `已存在相同 ID 的节点：${id}`;
    return false;
  }

  savedConfig.nodes.push({ id, url });
  renderSavedNodesTable();
  const feedback = el('proxy-save-feedback');
  if (feedback) feedback.textContent = `节点 ${id} 已加入草稿，请点击「保存配置并写入」落盘。`;
  return true;
}

/**
 * Save complete config to backend PUT /api/settings/network
 */
async function saveAllConfig() {
  const feedback = el('proxy-save-feedback');
  const btnSave = el('proxy-btn-save-all');
  if (btnSave) btnSave.disabled = true;
  if (feedback) feedback.textContent = '正在保存网络配置至 proxy.json…';

  try {
    const res = await parallax.saveNetworkSettings(savedConfig);
    pendingRestart = Boolean(res.pending_restart);
    renderRestartBanner();
    renderSavedNodesTable();
    if (feedback) {
      feedback.textContent = pendingRestart
        ? '✅ 配置已成功保存！由于 Rust 进程固定网络出口，请重启本地 38480 实例以加载新代理。'
        : '✅ 网络配置已成功保存！';
    }
    // Notify settings page if open
    window.dispatchEvent(new CustomEvent('parallax:network-settings-updated', { detail: savedConfig }));
  } catch (err) {
    if (feedback) feedback.textContent = `❌ 保存失败：${err.message}`;
  } finally {
    if (btnSave) btnSave.disabled = false;
  }
}

/**
 * Export current savedConfig as JSON file
 */
function exportConfigJson() {
  const jsonStr = JSON.stringify(savedConfig, null, 2);
  const blob = new Blob([jsonStr], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = 'parallax-network-config.json';
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

/**
 * Import JSON config from user file
 */
function initImportJson() {
  const fileInput = el('proxy-file-import');
  const btnImport = el('proxy-btn-import-json');
  if (!fileInput || !btnImport) return;

  btnImport.onclick = () => fileInput.click();

  fileInput.onchange = async () => {
    const file = fileInput.files?.[0];
    if (!file) return;
    const feedback = el('proxy-save-feedback');
    try {
      const text = await file.text();
      const parsed = JSON.parse(text);
      if (!parsed || typeof parsed !== 'object') throw new Error('无效的 JSON 对象');

      // Validate structure
      const newEnabled = typeof parsed.enabled === 'boolean' ? parsed.enabled : savedConfig.enabled;
      const newNodes = Array.isArray(parsed.nodes) ? parsed.nodes : [];
      const newRoutes = parsed.routes && typeof parsed.routes === 'object' ? parsed.routes : savedConfig.routes;

      if (newNodes.length > 20) throw new Error('导入失败：配置中包含超过 20 个节点。');

      savedConfig = {
        enabled: newEnabled,
        nodes: newNodes,
        routes: newRoutes
      };
      renderAll();
      if (feedback) feedback.textContent = '✅ 已从 JSON 成功读取网络配置草稿，点击「保存配置并写入」提交。';
    } catch (e) {
      if (feedback) feedback.textContent = `❌ 导入失败：${e.message}`;
    } finally {
      fileInput.value = '';
    }
  };
}

/**
 * Initialize event listeners
 */
function initProxyListeners() {
  // Refresh active
  const btnRefresh = el('proxy-btn-refresh-active');
  if (btnRefresh) btnRefresh.onclick = () => loadProxyData();

  // Save all
  const btnSave = el('proxy-btn-save-all');
  if (btnSave) btnSave.onclick = () => saveAllConfig();

  // Export JSON
  const btnExport = el('proxy-btn-export-json');
  if (btnExport) btnExport.onclick = () => exportConfigJson();

  // Toggle Add Drawer
  const btnToggleAdd = el('proxy-btn-toggle-add');
  const addDrawer = el('node-add-drawer');
  if (btnToggleAdd && addDrawer) {
    btnToggleAdd.onclick = () => {
      const isOpen = addDrawer.style.display !== 'none';
      addDrawer.style.display = isOpen ? 'none' : 'block';
      if (!isOpen) el('input-node-id')?.focus();
    };
  }

  // Cancel Add
  const btnCancelAdd = el('btn-node-cancel-add');
  if (btnCancelAdd && addDrawer) {
    btnCancelAdd.onclick = () => {
      addDrawer.style.display = 'none';
    };
  }

  // Confirm Add
  const btnConfirmAdd = el('btn-node-confirm-add');
  if (btnConfirmAdd) {
    btnConfirmAdd.onclick = () => {
      const idInput = el('input-node-id');
      const urlInput = el('input-node-url');
      if (addNode(idInput.value, urlInput.value)) {
        idInput.value = '';
        urlInput.value = '';
        if (addDrawer) addDrawer.style.display = 'none';
      }
    };
  }

  initImportJson();

  // Listen to hash change to load when entering #proxy
  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'proxy') loadProxyData();
  });
  if (location.hash === '#proxy') loadProxyData();
}

initProxyListeners();
loadProxyData().catch(console.error);
