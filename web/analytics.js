import { parallax } from './lib/api.js';
import { rNode } from './lib/research-view.js';

const el = id => document.getElementById(id);

function fillPreview(targetId, items, formatter) {
  const box = el(targetId);
  if (!box) return;
  box.replaceChildren();
  if (!items || !items.length) {
    box.append(rNode('p', 'hint', '当前尚未保存快照；点击上方入口即可实时拉取。'));
    return;
  }
  const list = rNode('div', 'analytics-snap-list');
  items.slice(0, 3).forEach(item => {
    const row = rNode('div', 'analytics-snap-row');
    const { title, meta } = formatter(item);
    row.append(rNode('strong', '', title), rNode('span', '', meta));
    list.append(row);
  });
  box.append(list);
}

export async function refreshAnalyticsHub() {
  try {
    const [booksRes, accRes] = await Promise.all([
      parallax.orderBooks().catch(() => ({ items: [] })),
      parallax.accounts().catch(() => ({ items: [] })),
    ]);

    const books = booksRes?.items || [];
    fillPreview('analytics-book-preview', books, b => ({
      title: b.request?.market_hash_name || '未知商品',
      meta: `${b.book?.currency || ''} · ${new Date((b.observed_at || 0) * 1000).toLocaleDateString()}`,
    }));
    if (el('analytics-book-count')) el('analytics-book-count').textContent = `${books.length} 份已存`;

    const primaryAcc = accRes?.items?.[0]?.id;
    if (primaryAcc) {
      const [buffRes, histRes] = await Promise.all([
        parallax.buffQuotes(primaryAcc).catch(() => ({ items: [] })),
        parallax.priceHistories(primaryAcc).catch(() => ({ items: [] })),
      ]);
      const buffItems = buffRes?.items || [];
      const histItems = histRes?.items || [];
      fillPreview('analytics-buff-preview', buffItems, q => ({
        title: q.request?.item_key || '未知饰品',
        meta: q.lowest_cash_cents != null ? `¥${(q.lowest_cash_cents / 100).toFixed(2)}` : '空卖盘',
      }));
      fillPreview('analytics-history-preview', histItems, h => ({
        title: h.request?.item_key || '未知饰品',
        meta: `${h.request?.window_days || 90} 日 · ${new Date((h.observed_at || 0) * 1000).toLocaleDateString()}`,
      }));
      if (el('analytics-buff-count')) el('analytics-buff-count').textContent = `${buffItems.length} 份已存`;
      if (el('analytics-history-count')) el('analytics-history-count').textContent = `${histItems.length} 份已存`;
    }
  } catch (err) {
    console.warn('Analytics hub refresh error:', err);
  }
}

window.addEventListener('parallax:pagechange', event => {
  if (event.detail?.id === 'analytics') {
    refreshAnalyticsHub();
  }
});

if (location.hash === '#analytics') {
  refreshAnalyticsHub();
}
