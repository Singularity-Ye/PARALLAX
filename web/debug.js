import { parallax } from './lib/api.js';
import { node } from './lib/pricing-view.js';

const el = id => document.getElementById(id);

const statusDict = {
  completed: { label: '已完成', cls: 'ok' },
  running: { label: '运行中', cls: 'warn' },
  cancelled: { label: '已取消', cls: 'muted' },
  failed: { label: '执行失败', cls: 'fail' }
};

const reasonDict = {
  all_candidates_examined: '所有候选均已比对完毕',
  source_exhausted: '候选数据源已全部遍历',
  cancelled_by_user: '用户主动中止',
  page_budget: '达到预设分页预算上限',
  budget_exhausted: '分析预算消耗完毕',
  network_timeout: '网络请求超时',
  server_error: '远程服务响应异常'
};

let allScans = [];
let isFetching = false;

function formatShortId(id) {
  if (!id || typeof id !== 'string') return '—';
  if (id.length <= 12) return id;
  return `${id.slice(0, 8)}…`;
}

async function loadScans() {
  const btnRefresh = el('scan-log-refresh');
  const statusEl = el('scan-log-status');
  if (isFetching) return;
  isFetching = true;
  if (btnRefresh) btnRefresh.disabled = true;
  if (statusEl) statusEl.textContent = '正在读取扫描流水…';

  try {
    const res = await parallax.scans();
    allScans = Array.isArray(res.items) ? res.items : [];
    renderKPIs();
    renderScansTable();
    if (statusEl) {
      statusEl.textContent = `共载入 ${allScans.length} 条扫描任务记录`;
    }
  } catch (err) {
    if (statusEl) {
      statusEl.textContent = `❌ 读取失败：${err.message}`;
    }
    const tbody = el('scan-log-tbody');
    if (tbody) {
      tbody.innerHTML = `<tr class="empty-row"><td colspan="7" class="empty-cell" style="color:#dc2626;">读取失败：${err.message}；请检查本地 38480 服务。</td></tr>`;
    }
  } finally {
    isFetching = false;
    if (btnRefresh) btnRefresh.disabled = false;
  }
}

function renderKPIs() {
  const totalEl = el('kpi-total-scans');
  const compEl = el('kpi-completed-scans');
  const examEl = el('kpi-examined-items');
  const accEl = el('kpi-accepted-items');

  const total = allScans.length;
  let completed = 0;
  let examined = 0;
  let accepted = 0;

  allScans.forEach(s => {
    if (s.status === 'completed') completed++;
    if (s.coverage) {
      examined += Number(s.coverage.examined || 0);
      accepted += Number(s.coverage.accepted || 0);
    }
  });

  if (totalEl) totalEl.textContent = total;
  if (compEl) compEl.textContent = completed;
  if (examEl) examEl.textContent = examined;
  if (accEl) accEl.textContent = accepted;
}

function getFilteredScans() {
  const filterVal = el('scan-log-filter')?.value || '';
  const searchVal = (el('scan-log-search')?.value || '').trim().toLowerCase();

  return allScans.filter(item => {
    if (filterVal && item.status !== filterVal) return false;
    if (searchVal) {
      const matchId = (item.id || '').toLowerCase().includes(searchVal);
      const reasonText = (reasonDict[item.stop_reason] || item.stop_reason || '').toLowerCase();
      const matchReason = reasonText.includes(searchVal);
      const matchSource = (item.request?.source || '').toLowerCase().includes(searchVal);
      if (!matchId && !matchReason && !matchSource) return false;
    }
    return true;
  });
}

function renderScansTable() {
  const tbody = el('scan-log-tbody');
  const exportBtn = el('scan-log-export');
  if (!tbody) return;

  tbody.replaceChildren();
  const items = getFilteredScans();

  if (exportBtn) exportBtn.disabled = items.length === 0;

  if (items.length === 0) {
    const tr = node('tr');
    tr.className = 'empty-row';
    const td = node('td', allScans.length === 0 ? '本地暂无扫描记录。前往选品工作台点击「启动选品扫描」创建任务。' : '当前筛选条件下没有匹配的任务。');
    td.colSpan = 7;
    td.className = 'empty-cell';
    tr.append(td);
    tbody.append(tr);
    return;
  }

  items.forEach(report => {
    const tr = node('tr');

    // 1. Time
    const tdTime = node('td');
    tdTime.textContent = report.created_at ? new Date(report.created_at * 1000).toLocaleString() : '—';
    tdTime.style.whiteSpace = 'nowrap';
    tdTime.style.color = '#334155';

    // 2. Scan ID
    const tdId = node('td');
    const idTag = node('span', formatShortId(report.id));
    idTag.className = 'scan-id-tag';
    idTag.title = report.id || '';
    tdId.append(idTag);

    // 3. Source
    const tdSource = node('td');
    const isDemo = report.request?.source === 'demo';
    const srcBadge = node('span', isDemo ? '演示 (Demo)' : 'SteamDT');
    srcBadge.className = `badge ${isDemo ? 'muted' : 'ok'}`;
    tdSource.append(srcBadge);

    // 4. Status
    const tdStatus = node('td');
    const stConfig = statusDict[report.status] || { label: report.status || '未知', cls: 'muted' };
    const stBadge = node('span', stConfig.label);
    stBadge.className = `badge ${stConfig.cls}`;
    tdStatus.append(stBadge);

    // 5. Reason / Diagnosis
    const tdReason = node('td');
    const reasonText = reasonDict[report.stop_reason] || report.stop_reason || '正常执行';
    tdReason.textContent = reasonText;
    tdReason.style.fontSize = '11px';

    // 6. Coverage Bar (accepted / examined)
    const tdCoverage = node('td');
    const cov = report.coverage || { accepted: 0, examined: 0 };
    const acc = Number(cov.accepted || 0);
    const exam = Number(cov.examined || 0);
    const pct = exam > 0 ? Math.round((acc / exam) * 100) : 0;

    const covWrap = node('div');
    covWrap.className = 'coverage-bar-wrap';

    const track = node('div');
    track.className = 'coverage-track';
    const fill = node('div');
    fill.className = 'coverage-fill';
    fill.style.width = `${Math.min(pct, 100)}%`;
    track.append(fill);

    const txt = node('span', `${acc}/${exam} (${pct}%)`);
    txt.className = 'coverage-text';

    covWrap.append(track, txt);
    tdCoverage.append(covWrap);

    // 7. Action
    const tdAction = node('td');
    tdAction.style.textAlign = 'right';
    const viewBtn = node('a', '选品台 ↗');
    viewBtn.className = 'table-action-link';
    viewBtn.href = '#catalog';
    tdAction.append(viewBtn);

    tr.append(tdTime, tdId, tdSource, tdStatus, tdReason, tdCoverage, tdAction);
    tbody.append(tr);
  });
}

function exportScansJson() {
  const visibleItems = getFilteredScans();
  const blob = new Blob([JSON.stringify(visibleItems, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = `parallax-scans-log-${Date.now()}.json`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

function initDebug() {
  const filterSelect = el('scan-log-filter');
  if (filterSelect) {
    // Populate filter options if empty
    if (filterSelect.options.length <= 1) {
      for (const [val, cfg] of Object.entries(statusDict)) {
        const opt = document.createElement('option');
        opt.value = val;
        opt.textContent = cfg.label;
        filterSelect.appendChild(opt);
      }
    }
    filterSelect.onchange = () => renderScansTable();
  }

  const searchInput = el('scan-log-search');
  if (searchInput) {
    searchInput.oninput = () => renderScansTable();
  }

  const btnRefresh = el('scan-log-refresh');
  if (btnRefresh) btnRefresh.onclick = () => loadScans();

  const btnExport = el('scan-log-export');
  if (btnExport) btnExport.onclick = () => exportScansJson();

  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'debug') loadScans();
  });

  if (location.hash === '#debug') loadScans();
}

initDebug();
loadScans().catch(console.error);
