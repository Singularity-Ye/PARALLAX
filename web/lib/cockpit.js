// Dashboard Executive Portfolio & Quant Cockpit.
import {parallax} from './api.js';
import {node} from './pricing-view.js';

const el = id => document.getElementById(id);
const amount = value => Number.isSafeInteger(value) && value >= 0 ? `¥${(value / 100).toFixed(2)}` : '未知';
const ratio = value => Number.isFinite(value) ? `${(value / 100).toFixed(2)}%` : '未知';
const stageLabels = {
  pending_receipt: '待收货',
  held: '持有中',
  listed: '已挂牌',
  sold: '已结算'
};

let scan = null;
let planRevision = 0;

export function renderCockpitScan(report) {
  scan = report;
  const rows = report?.candidates ?? [];
  const emptyEl = el('dashboard-empty-candidates');
  const tableEl = el('dashboard-candidates-table');
  if (emptyEl) {
    emptyEl.hidden = rows.length > 0;
    emptyEl.style.display = rows.length ? 'none' : 'flex';
  }
  if (tableEl) {
    tableEl.hidden = rows.length === 0;
    tableEl.style.display = rows.length ? 'table' : 'none';
  }

  const passEl = el('dashboard-pass-rate');
  if (passEl) {
    const acc = report?.coverage?.accepted ?? 0;
    const exam = report?.coverage?.examined ?? 0;
    passEl.textContent = exam > 0 ? `通过率 ${Math.round((acc / exam) * 100)}%` : '命中率: —';
    passEl.title = `预筛通过 ${acc} 件 / 共检验 ${exam} 件`;
  }
  if (el('dashboard-selection') && report?.coverage) {
    el('dashboard-selection').textContent = `${report.coverage.accepted ?? 0} / ${report.coverage.examined ?? 0}`;
  }
  if (el('dashboard-source') && report?.request) {
    el('dashboard-source').textContent = report.request.source === 'demo' ? '演示样本' : 'SteamDT 实时榜';
  }

  const tbody = el('dashboard-candidates-tbody');
  if (!tbody) return;
  const fragment = document.createDocumentFragment();
  for (const c of rows.slice(0, 7)) {
    const d = c.deep_analysis || {};
    const row = node('tr');
    const volText = `${c.item.volume}${d.sell_pressure !== undefined ? ` · 卖压 ${d.sell_pressure.toFixed(1)}` : ''}`;
    const passText = c.passed ? `通过 (限 ${d.safe_purchase_limit ?? '—'}件)` : '未通过';

    const nameTd = node('td');
    const nameStrong = node('strong', c.item.name);
    nameStrong.style.color = 'var(--ink)';
    nameTd.append(nameStrong);

    const steamTd = node('td', c.item.steam_gross_cents > 0 ? amount(c.item.steam_gross_cents) : '未知');
    const buffTd = node('td', c.item.cash_cents > 0 ? amount(c.item.cash_cents) : '未知');
    const ratioTd = node('td');
    const ratioBadge = node('span', ratio(c.ratio_bps));
    ratioBadge.className = c.passed ? 'status-pill status-pill-emerald' : 'status-pill';
    ratioTd.append(ratioBadge);

    const volTd = node('td', volText);
    const passTd = node('td', passText);
    passTd.style.color = c.passed ? '#15803d' : 'var(--muted)';
    passTd.style.fontWeight = c.passed ? '700' : '500';

    const action = node('td');
    const link = node('a', '配平 ↗');
    link.href = '#planning';
    link.className = 'text-link';
    action.append(link);

    row.append(nameTd, steamTd, buffTd, ratioTd, volTd, passTd, action);
    fragment.append(row);
  }
  tbody.replaceChildren(fragment);
}

function renderPlan(plan) {
  const budget = plan.request.budget_cents;
  const planned = plan.cash_total_cents;
  const reserved = plan.reserved_cash_cents ?? 0;
  const remaining = plan.remaining_budget_cents;
  const valid = [budget, planned, reserved, remaining].every(v => Number.isSafeInteger(v) && v >= 0) && budget > 0 && planned + reserved + remaining === budget;

  if (el('dashboard-budget-val')) el('dashboard-budget-val').textContent = amount(budget);
  if (el('dashboard-wallet-val')) el('dashboard-wallet-val').textContent = amount(plan.estimated_wallet_total_cents);
  if (el('dashboard-reserved-val')) {
    el('dashboard-reserved-val').textContent = `动用 ${amount(planned)} · 剩余 ${amount(remaining)}`;
  }

  const ratioPill = el('dashboard-plan-ratio-pill');
  if (ratioPill) {
    if (planned > 0 && plan.estimated_wallet_total_cents > 0) {
      const r = (planned / plan.estimated_wallet_total_cents).toFixed(3);
      ratioPill.textContent = `折算 ${r}`;
    } else {
      ratioPill.textContent = '待动用预算';
    }
  }

  const totalCapEl = el('telemetry-total-capital');
  const barAvail = el('alloc-bar-avail');
  const barResv = el('alloc-bar-resv');
  const barHold = el('alloc-bar-hold');
  const legAvail = el('legend-avail-val');
  const legResv = el('legend-resv-val');
  const legHold = el('legend-hold-val');

  if (totalCapEl) totalCapEl.textContent = `计划总额度: ${amount(budget)}`;
  if (legAvail) legAvail.textContent = amount(remaining);
  if (legResv) legResv.textContent = amount(reserved);
  if (legHold) legHold.textContent = amount(planned);
  if (barAvail && valid) barAvail.style.width = `${Math.max(0, (remaining / budget) * 100)}%`;
  if (barResv && valid) barResv.style.width = `${Math.max(0, (reserved / budget) * 100)}%`;
  if (barHold && valid) barHold.style.width = `${Math.max(0, (planned / budget) * 100)}%`;

  if (el('dashboard-plan-scope')) {
    el('dashboard-plan-scope').textContent = `最新配平计划 · 生成于 ${new Date(plan.created_at * 1000).toLocaleTimeString()} · 单品上限 ${plan.request.max_units_per_item} 件`;
  }

  const position = plan.request.position_snapshot;
  const isAuto = plan.position_source === 'auto_inventory_and_ledger';
  const positionText = isAuto
    ? '自动关联当前账户库存与台账'
    : (position ? '引用已保存仓位快照' : '独立测算模式');
  if (el('sentinel-position')) el('sentinel-position').textContent = positionText;

  const policy = plan.request.liquidity_policy;
  if (el('sentinel-risk')) {
    el('sentinel-risk').textContent = policy
      ? `日量 5% 约束${policy.volume_limit_enabled !== false ? '已启用' : '关闭'} · 低价折半保护${policy.low_price_guard_enabled !== false ? '已启用' : '关闭'}`
      : '默认安全限购约束已就绪';
  }
}

async function refreshPortfolioAndWorkers() {
  // 1. Load Accounts & Active Account Holdings Ledger
  try {
    const accRes = await parallax.accounts();
    const accounts = accRes?.items ?? [];
    if (accounts.length > 0) {
      const primary = accounts[0];
      try {
        const prof = await parallax.accountProfile(primary.id);
        if (prof && el('hero-acc-region')) {
          const reg = prof.region_code || 'UA';
          const cur = prof.currency_code || 'UAH';
          const bal = prof.wallet_balance_display ? ` · ${prof.wallet_balance_display}` : '';
          el('hero-acc-region').textContent = `${reg} · ${cur}${bal}`;
        }
      } catch (_) {}

      try {
        const [summary, rowsPage] = await Promise.all([
          parallax.ledgerSummary(primary.id),
          parallax.ledgerRows(primary.id, { offset: 0, limit: 6 })
        ]);
        if (summary) {
          const costMinor = summary.open_purchase_cost?.minor ?? 0;
          if (el('dashboard-holdings-cost')) {
            el('dashboard-holdings-cost').textContent = amount(costMinor);
          }
          if (el('dashboard-holdings-count')) {
            el('dashboard-holdings-count').textContent = `${summary.counts?.held_units ?? 0} 件在持`;
          }
          if (el('dashboard-holdings-listed')) {
            el('dashboard-holdings-listed').textContent = `已挂牌 ${summary.counts?.listed_units ?? 0} 件 · 已结算 ${summary.counts?.sold_units ?? 0} 件`;
          }
        }
        renderDashboardHoldings(rowsPage?.items ?? []);
      } catch (_) {}
    }
  } catch (_) {}

  // 2. Load Background Workers Status
  try {
    const wRes = await parallax.workers();
    const workers = wRes?.workers ?? [];
    const mapBadge = (name, id) => {
      const w = workers.find(item => item.name === name);
      const badge = el(id);
      if (!w || !badge) return;
      badge.textContent = w.running ? '执行中' : (w.run_count > 0 ? `已运行 ${w.run_count} 次` : '空闲就绪');
      badge.className = 'status-badge online';
    };
    mapBadge('listing_reconciliation', 'sw-badge-listing');
    mapBadge('fx_refresh', 'sw-badge-fx');
    mapBadge('account_sync', 'sw-badge-account');
  } catch (_) {}
}

function renderDashboardHoldings(items) {
  const emptyEl = el('dashboard-empty-holdings');
  const tableEl = el('dashboard-holdings-table');
  const tbody = el('dashboard-holdings-tbody');
  if (!tbody) return;

  if (!items || items.length === 0) {
    if (emptyEl) emptyEl.style.display = 'flex';
    if (tableEl) tableEl.style.display = 'none';
    return;
  }
  if (emptyEl) emptyEl.style.display = 'none';
  if (tableEl) tableEl.style.display = 'table';

  const frag = document.createDocumentFragment();
  for (const row of items.slice(0, 6)) {
    const tr = node('tr');
    const qty = Math.max(1, Number(row.quantity || 1));
    const costCents = row.purchase_cost?.minor ?? 0;
    const estNetCents = row.sale_net_wallet?.value?.minor ?? Math.round((costCents / 0.72) * 0.8696);

    const nameTd = node('td');
    const strong = node('strong', row.item_key);
    strong.style.color = 'var(--ink)';
    nameTd.append(strong);

    const qtyTd = node('td', `${qty} 件`);
    const costTd = node('td', amount(costCents));
    const netTd = node('td', amount(estNetCents));
    netTd.style.color = '#15803d';
    netTd.style.fontWeight = '700';

    const stageTd = node('td');
    const pill = node('span', stageLabels[row.stage] || row.stage);
    pill.className = row.stage === 'listed' || row.stage === 'sold' ? 'status-pill status-pill-emerald' : 'status-pill';
    stageTd.append(pill);

    tr.append(nameTd, qtyTd, costTd, netTd, stageTd);
    frag.append(tr);
  }
  tbody.replaceChildren(frag);
}

export function initCockpit() {
  window.addEventListener('parallax:plan-selected', ({ detail }) => {
    ++planRevision;
    renderPlan(detail);
  });

  window.addEventListener('parallax:pagechange', ({ detail }) => {
    if (detail?.id === 'dashboard') {
      refreshPortfolioAndWorkers();
    }
  });

  window.addEventListener('parallax:account-created', () => {
    refreshPortfolioAndWorkers();
  });

  const revision = planRevision;
  parallax.plans().then(async ({ items }) => {
    if (!items.length) return;
    const latest = [...items].sort((a, b) => b.created_at - a.created_at)[0];
    const saved = await parallax.plan(latest.id);
    if (planRevision === revision) renderPlan(saved);
  }).catch(() => {});

  parallax.scans().then(async ({ items }) => {
    if (scan || !items?.length) return;
    const latest = [...items].sort((a, b) => b.created_at - a.created_at)[0];
    if (latest) {
      const fullScan = await parallax.scan(latest.id);
      if (!scan && fullScan) renderCockpitScan(fullScan);
    }
  }).catch(() => {});

  parallax.proxyStatus().then(value => {
    if (el('sentinel-route')) {
      el('sentinel-route').textContent = value.mode === 'explicit_routes' ? '已启用平台分流代理' : '直连网络（遵循系统环境）';
    }
  }).catch(() => {});

  refreshPortfolioAndWorkers();
}
