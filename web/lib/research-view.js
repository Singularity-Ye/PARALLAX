import { createItemThumb } from './icon-helper.js';

export function rNode(tag, className, text) {
  const n = document.createElement(tag);
  if (className) n.className = className;
  if (text !== undefined) n.textContent = text;
  return n;
}

export function renderResearchBanner(itemKey, subtitleText, actions = []) {
  const card = rNode('div', 'research-item-banner');
  const left = rNode('div', 'research-banner-left');
  left.append(createItemThumb(itemKey, null, 'md'));
  const meta = rNode('div', 'research-banner-meta');
  meta.append(rNode('h3', '', itemKey || '未知饰品'), rNode('p', 'hint', subtitleText));
  left.append(meta);
  card.append(left);
  if (actions.length) {
    const right = rNode('div', 'research-banner-actions');
    actions.forEach(a => right.append(a));
    card.append(right);
  }
  return card;
}

export function renderKpiGrid(tiles) {
  const grid = rNode('div', 'research-kpi-grid');
  for (const t of tiles) {
    const tile = rNode('div', `research-kpi-tile ${t.tone || ''}`.trim());
    tile.append(
      rNode('span', 'research-kpi-label', t.label),
      rNode('strong', 'research-kpi-val', t.value ?? '—'),
    );
    if (t.hint) tile.append(rNode('small', 'research-kpi-hint', t.hint));
    grid.append(tile);
  }
  return grid;
}

export function renderDepthCard(title, kind, levels, formatPrice, emptyText = '暂无挂单档位') {
  const card = rNode('div', 'research-panel-card');
  const head = rNode('div', 'research-card-head');
  const tag = rNode('span', `badge ${kind === 'bid' ? 'ok' : 'muted'}`, kind === 'bid' ? '买盘需求' : '卖盘供给');
  head.append(rNode('h4', '', title), tag);
  card.append(head);

  const slice = (levels || []).slice(0, 10);
  if (!slice.length) {
    card.append(rNode('p', 'hint', emptyText));
    return card;
  }

  const maxQty = Math.max(1, ...slice.map(l => Number(l.quantity || 0)));
  const tbl = rNode('table', 'cockpit-table research-depth-table');
  tbl.innerHTML = '<thead><tr><th>档位</th><th>挂单价格</th><th style="text-align:right">数量</th><th style="width:92px">深度占比</th></tr></thead>';
  const tbody = document.createElement('tbody');

  slice.forEach((lvl, idx) => {
    const tr = document.createElement('tr');
    const qty = Number(lvl.quantity || 0);
    const pct = Math.max(4, Math.round((qty / maxQty) * 100));

    const tdRank = rNode('td', 'col-mono', `#${idx + 1}`);
    const tdPrice = rNode('td', 'col-mono', formatPrice(lvl.price_minor));
    const tdQty = rNode('td', 'col-mono', String(qty));
    tdQty.style.textAlign = 'right';

    const tdBar = document.createElement('td');
    const track = rNode('div', 'depth-bar-track');
    const fill = rNode('div', `depth-bar-fill ${kind === 'bid' ? 'is-bid' : 'is-ask'}`);
    fill.style.width = `${pct}%`;
    track.append(fill);
    tdBar.append(track);

    tr.append(tdRank, tdPrice, tdQty, tdBar);
    tbody.append(tr);
  });

  tbl.append(tbody);
  card.append(tbl);
  return card;
}

const fmtDay = d => new Date(d * 86400000).toISOString().slice(5, 10);

export function renderPriceHistoryChart(dailyBars, cleanMeanMinor, formatMoney) {
  const card = rNode('div', 'research-panel-card history-chart-card');
  const head = rNode('div', 'research-card-head');
  const legend = rNode('div', 'chart-legend');
  legend.innerHTML = '<span class="legend-chip"><i class="dot-line"></i>日均 VWAP</span><span class="legend-chip"><i class="dot-mean"></i>清洗均价</span><span class="legend-chip"><i class="dot-vol"></i>成交量</span>';
  head.append(rNode('h4', '', '历史量价走势 (UTC 日线)'), legend);
  card.append(head);

  const validBars = (dailyBars || []).filter(b => b.vwap && Number.isFinite(b.vwap.minor));
  if (validBars.length < 2) {
    card.append(rNode('p', 'hint', validBars.length === 1 ? '仅有 1 个有效交易日，样本不足以绘制连续走势曲线。' : '暂无足够的有效日价格点以生成趋势图。'));
    return card;
  }

  const W = 720, H = 210;
  const padL = 62, padR = 16, padT = 16, padB = 28;
  const plotW = W - padL - padR;
  const plotH = H - padT - padB;

  const prices = validBars.map(b => b.vwap.minor);
  if (cleanMeanMinor && Number.isFinite(cleanMeanMinor)) prices.push(cleanMeanMinor);
  let minP = Math.min(...prices);
  let maxP = Math.max(...prices);
  if (maxP === minP) {
    maxP += Math.max(10, Math.round(maxP * 0.08));
    minP = Math.max(0, minP - Math.max(10, Math.round(minP * 0.08)));
  } else {
    const pad = Math.round((maxP - minP) * 0.12);
    minP = Math.max(0, minP - pad);
    maxP += pad;
  }

  const maxVol = Math.max(1, ...validBars.map(b => Number(b.volume || 0)));
  const xFor = i => padL + (i / (validBars.length - 1)) * plotW;
  const yFor = p => padT + (1 - (p - minP) / Math.max(1, maxP - minP)) * plotH;

  const NS = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(NS, 'svg');
  svg.setAttribute('viewBox', `0 0 ${W} ${H}`);
  svg.setAttribute('class', 'history-svg-chart');

  const defs = document.createElementNS(NS, 'defs');
  const grad = document.createElementNS(NS, 'linearGradient');
  grad.setAttribute('id', 'parallax-hist-fill');
  grad.setAttribute('x1', '0'); grad.setAttribute('y1', '0');
  grad.setAttribute('x2', '0'); grad.setAttribute('y2', '1');
  const s1 = document.createElementNS(NS, 'stop');
  s1.setAttribute('offset', '0%'); s1.setAttribute('stop-color', '#5f704e'); s1.setAttribute('stop-opacity', '0.22');
  const s2 = document.createElementNS(NS, 'stop');
  s2.setAttribute('offset', '100%'); s2.setAttribute('stop-color', '#5f704e'); s2.setAttribute('stop-opacity', '0.01');
  grad.append(s1, s2);
  defs.append(grad);
  svg.append(defs);

  // Horizontal Y grid & labels
  for (let i = 0; i <= 3; i++) {
    const val = minP + ((maxP - minP) * i) / 3;
    const y = yFor(val);
    const line = document.createElementNS(NS, 'line');
    line.setAttribute('x1', String(padL)); line.setAttribute('x2', String(W - padR));
    line.setAttribute('y1', y.toFixed(1)); line.setAttribute('y2', y.toFixed(1));
    line.setAttribute('stroke', '#e5e9dc'); line.setAttribute('stroke-dasharray', i === 0 ? '' : '3 3');
    svg.append(line);

    const lbl = document.createElementNS(NS, 'text');
    lbl.setAttribute('x', String(padL - 8)); lbl.setAttribute('y', (y + 3.5).toFixed(1));
    lbl.setAttribute('text-anchor', 'end'); lbl.setAttribute('fill', '#6b7263'); lbl.setAttribute('font-size', '10');
    lbl.textContent = formatMoney({ minor: Math.round(val), currency: validBars[0].vwap.currency });
    svg.append(lbl);
  }

  // Volume bars along bottom 26%
  const barW = Math.max(2, Math.min(10, plotW / validBars.length - 1.5));
  validBars.forEach((b, i) => {
    const vol = Number(b.volume || 0);
    if (vol <= 0) return;
    const h = (vol / maxVol) * (plotH * 0.26);
    const rect = document.createElementNS(NS, 'rect');
    rect.setAttribute('x', (xFor(i) - barW / 2).toFixed(1));
    rect.setAttribute('y', (padT + plotH - h).toFixed(1));
    rect.setAttribute('width', barW.toFixed(1));
    rect.setAttribute('height', h.toFixed(1));
    rect.setAttribute('rx', '1');
    rect.setAttribute('fill', '#d5dec7');
    svg.append(rect);
  });

  // Clean mean dashed line
  if (cleanMeanMinor && Number.isFinite(cleanMeanMinor)) {
    const ym = yFor(cleanMeanMinor);
    const mLine = document.createElementNS(NS, 'line');
    mLine.setAttribute('x1', String(padL)); mLine.setAttribute('x2', String(W - padR));
    mLine.setAttribute('y1', ym.toFixed(1)); mLine.setAttribute('y2', ym.toFixed(1));
    mLine.setAttribute('stroke', '#a67b3d'); mLine.setAttribute('stroke-width', '1.2');
    mLine.setAttribute('stroke-dasharray', '5 4');
    svg.append(mLine);
  }

  // Area & Price curve
  const pts = validBars.map((b, i) => `${xFor(i).toFixed(1)},${yFor(b.vwap.minor).toFixed(1)}`);
  const area = document.createElementNS(NS, 'polygon');
  area.setAttribute('points', `${padL},${padT + plotH} ${pts.join(' ')} ${(padL + plotW).toFixed(1)},${padT + plotH}`);
  area.setAttribute('fill', 'url(#parallax-hist-fill)');
  svg.append(area);

  const polyline = document.createElementNS(NS, 'polyline');
  polyline.setAttribute('points', pts.join(' '));
  polyline.setAttribute('fill', 'none');
  polyline.setAttribute('stroke', '#48563c');
  polyline.setAttribute('stroke-width', '2');
  polyline.setAttribute('stroke-linejoin', 'round');
  svg.append(polyline);

  // Sample points with tooltips
  validBars.forEach((b, i) => {
    const cx = xFor(i), cy = yFor(b.vwap.minor);
    const dot = document.createElementNS(NS, 'circle');
    dot.setAttribute('cx', cx.toFixed(1)); dot.setAttribute('cy', cy.toFixed(1));
    dot.setAttribute('r', validBars.length > 45 ? '2.1' : '3');
    dot.setAttribute('fill', '#48563c'); dot.setAttribute('stroke', '#ffffff'); dot.setAttribute('stroke-width', '1');
    const title = document.createElementNS(NS, 'title');
    title.textContent = `${new Date(b.utc_day * 86400000).toISOString().slice(0, 10)} · 均价 ${formatMoney(b.vwap)} · 成交 ${b.volume} 件`;
    dot.append(title);
    svg.append(dot);
  });

  // X-axis date ticks (start, mid, end)
  const ticks = [0, Math.floor((validBars.length - 1) / 2), validBars.length - 1];
  [...new Set(ticks)].forEach((idx, pos) => {
    const lbl = document.createElementNS(NS, 'text');
    lbl.setAttribute('x', xFor(idx).toFixed(1));
    lbl.setAttribute('y', String(H - 7));
    lbl.setAttribute('text-anchor', pos === 0 ? 'start' : idx === validBars.length - 1 ? 'end' : 'middle');
    lbl.setAttribute('fill', '#6b7263'); lbl.setAttribute('font-size', '10');
    lbl.textContent = fmtDay(validBars[idx].utc_day);
    svg.append(lbl);
  });

  card.append(svg);
  return card;
}

const fmtScanTime = ts => {
  if (!ts) return '未知时间';
  const d = new Date(ts * 1000);
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const mi = String(d.getMinutes()).padStart(2, '0');
  return `${mm}/${dd} ${hh}:${mi}`;
};

export function formatScanOptionLabel(report, blockedPlatformLabel = '平台') {
  if (!report) return '未选择扫描';
  const src = report.request?.source === 'demo' ? '演示样本' : 'SteamDT 真实榜单';
  const dt = fmtScanTime(report.created_at);
  if (report.request?.source === 'demo') {
    return `${src} · ${dt}（不可查 ${blockedPlatformLabel}）`;
  }
  if (report.status && report.status !== 'completed') {
    const statusText = { running: '扫描中', failed: '已失败', cancelled: '已停止', interrupted: '已中断' }[report.status] || report.status;
    return `${src} · ${dt}（${statusText}）`;
  }
  return `${src} · ${dt} · 通过 ${report.coverage?.accepted ?? 0} 件`;
}

export function syncScanSelectOptions(selectEl, scans, currentReport, blockedPlatformLabel, extraTopOption = null) {
  if (!selectEl) return;
  const list = Array.isArray(scans) ? [...scans] : [];
  if (currentReport?.id) {
    const idx = list.findIndex(s => s.id === currentReport.id);
    if (idx >= 0) list[idx] = currentReport;
    else list.unshift(currentReport);
  }
  selectEl.replaceChildren();
  if (extraTopOption) {
    const opt = document.createElement('option');
    opt.value = extraTopOption.value;
    opt.textContent = extraTopOption.label;
    selectEl.append(opt);
  }
  if (!list.length && !extraTopOption) {
    const opt = document.createElement('option');
    opt.value = '';
    opt.textContent = '暂无选品扫描记录';
    opt.disabled = true;
    selectEl.append(opt);
    return;
  }
  for (const r of list) {
    const opt = document.createElement('option');
    opt.value = r.id;
    opt.textContent = formatScanOptionLabel(r, blockedPlatformLabel);
    selectEl.append(opt);
  }
  if (extraTopOption) {
    selectEl.value = extraTopOption.value;
  } else if (currentReport?.id && [...selectEl.options].some(o => o.value === currentReport.id)) {
    selectEl.value = currentReport.id;
  }
}
