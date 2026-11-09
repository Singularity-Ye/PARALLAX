import { parallax } from './lib/api.js';
import { node } from './lib/pricing-view.js';

const el = id => document.getElementById(id);

let allAccounts = [];
let currentFriends = [];
let currentEditions = [];
let isChecking = false;

async function loadGiftAccounts() {
  const select = el('gift-sender-account');
  if (!select) return;

  try {
    const res = await parallax.accounts();
    allAccounts = Array.isArray(res) ? res : (res?.accounts || []);
    select.replaceChildren();

    if (allAccounts.length === 0) {
      const opt = node('option', '暂无已配置账号，请先在账号管理中添加');
      opt.value = '';
      select.append(opt);
      return;
    }

    allAccounts.forEach(a => {
      const opt = node('option', `${a.label} (${a.steam_id || '未绑定SteamID'})`);
      opt.value = a.id;
      select.append(opt);
    });

    select.onchange = () => loadFriendsForCurrentAccount();
    loadFriendsForCurrentAccount();
  } catch (err) {
    console.warn('Failed to load gift accounts:', err);
  }
}

async function loadFriendsForCurrentAccount() {
  const select = el('gift-sender-account');
  const friendsContainer = el('gift-friends-container');
  const statusEl = el('gift-friends-status');
  const accountId = select?.value;

  if (!accountId) return;
  if (statusEl) statusEl.textContent = '正在从 Steam 安全拉取好友列表…';
  if (friendsContainer) friendsContainer.replaceChildren();

  try {
    const res = await parallax.accountFriends(accountId);
    currentFriends = res?.friends || [];
    if (statusEl) statusEl.textContent = `共载入 ${currentFriends.length} 位 Steam 好友`;
    renderFriendsList();
  } catch (err) {
    if (statusEl) statusEl.textContent = `⚠️ 读取好友失败：${err.message}`;
  }
}

function renderFriendsList() {
  const container = el('gift-friends-container');
  if (!container) return;
  container.replaceChildren();

  if (currentFriends.length === 0) {
    const emptyNotice = node('div', '暂未拉取到好友（可能 Cookie 已过期或无公共好友，可手动输入受赠 SteamID）');
    emptyNotice.className = 'route-desc-hint';
    container.append(emptyNotice);
    return;
  }

  currentFriends.slice(0, 15).forEach(f => {
    const item = node('div');
    item.className = 'route-selector-card';
    item.style.display = 'flex';
    item.style.alignItems = 'center';
    item.style.gap = '8px';
    item.style.padding = '6px 10px';
    item.style.cursor = 'pointer';

    const avatar = node('img');
    avatar.src = f.avatar;
    avatar.style.width = '28px';
    avatar.style.height = '28px';
    avatar.style.borderRadius = '50%';

    const info = node('div');
    const name = node('div', f.name);
    name.style.fontSize = '12px';
    name.style.fontWeight = '600';
    name.style.color = '#0f172a';
    const sid = node('div', f.steamid);
    sid.className = 'route-desc-hint';
    sid.style.fontSize = '10px';
    info.append(name, sid);

    item.append(avatar, info);
    item.onclick = () => {
      const recipientInput = el('gift-recipient-steamid');
      if (recipientInput) recipientInput.value = f.steamid;

      // Highlight selected card
      Array.from(container.children).forEach(c => c.style.borderColor = '#e2e8f0');
      item.style.borderColor = '#3b82f6';
    };

    container.append(item);
  });
}

async function detectEditions() {
  const appidInput = el('gift-appid-input');
  const editionSelect = el('gift-edition-select');
  const statusEl = el('gift-edition-status');
  const appid = Number(appidInput?.value || 0);

  if (!appid || appid <= 0) {
    if (statusEl) statusEl.textContent = '⚠️ 请先输入目标游戏的 Steam AppID';
    return;
  }

  if (statusEl) statusEl.textContent = '正在探测该游戏的售卖版本与 SubID…';
  if (editionSelect) editionSelect.replaceChildren();

  try {
    const res = await parallax.gameEditions(appid);
    currentEditions = res?.editions || [];
    if (statusEl) statusEl.textContent = `✓ 已探测到 ${res.title} 的 ${currentEditions.length} 个售卖版本`;

    currentEditions.forEach(ed => {
      const opt = node('option', `${ed.name} · ${ed.price} (SubID: ${ed.id})`);
      opt.value = ed.id;
      editionSelect.append(opt);
    });
  } catch (err) {
    if (statusEl) statusEl.textContent = `❌ 探测版本失败：${err.message}`;
  }
}

async function checkGiftRisk() {
  if (isChecking) return;
  const btn = el('btn-check-gift');
  const feedback = el('gift-risk-feedback');
  const resultCard = el('gift-risk-card');

  const senderAccount = el('gift-sender-account')?.value;
  const recipientSteamId = el('gift-recipient-steamid')?.value?.trim();
  const appid = Number(el('gift-appid-input')?.value || 0);
  const editionId = el('gift-edition-select')?.value;
  const recipientRegion = el('gift-recipient-region')?.value || 'CN';

  if (!senderAccount || !appid || !editionId) {
    if (feedback) feedback.textContent = '⚠️ 请完整选择赠送账号、输入游戏 AppID 并选择购买版本';
    return;
  }

  isChecking = true;
  if (btn) btn.disabled = true;
  if (feedback) feedback.textContent = '正在向 Steam 查询双方区域价格并执行 10% 价差风控测算…';

  try {
    const payload = {
      appid,
      edition_id: editionId,
      sender_account: senderAccount,
      recipient_steamid: recipientSteamId || '0',
      recipient_region: recipientRegion,
    };

    const res = await parallax.checkGift(payload);
    renderRiskVerdict(res);
    if (feedback) feedback.textContent = '✓ 测算完成！请查看下方安全风控裁定。';
  } catch (err) {
    if (feedback) feedback.textContent = `❌ 测算失败：${err.message}`;
  } finally {
    isChecking = false;
    if (btn) btn.disabled = false;
  }
}

function renderRiskVerdict(v) {
  const card = el('gift-risk-card');
  if (!card) return;
  card.style.display = 'block';

  const verdictBadge = el('gift-verdict-badge');
  const msgEl = el('gift-verdict-msg');
  const senderPriceEl = el('gift-sender-cny');
  const recipientPriceEl = el('gift-recipient-cny');
  const diffPctEl = el('gift-diff-pct');
  const storeLink = el('gift-store-link');

  if (v.verdict === 'PASS' || v.verdict === 'SAME_REGION') {
    verdictBadge.textContent = '✓ 跨区合规安全';
    verdictBadge.className = 'badge ok';
  } else if (v.verdict === 'REJECT_RISK') {
    verdictBadge.textContent = '⚠️ 触碰 10% 拒单红线';
    verdictBadge.className = 'badge fail';
  } else {
    verdictBadge.textContent = '未知 / 需客户端复核';
    verdictBadge.className = 'badge muted';
  }

  if (msgEl) msgEl.textContent = v.message;
  if (senderPriceEl) senderPriceEl.textContent = `¥${v.sender_price_cny.toFixed(2)} (${v.sender_region})`;
  if (recipientPriceEl) recipientPriceEl.textContent = `¥${v.recipient_price_cny.toFixed(2)} (${v.recipient_region})`;
  if (diffPctEl) {
    diffPctEl.textContent = `${v.diff_pct > 0 ? '+' : ''}${v.diff_pct}%`;
    diffPctEl.style.color = v.diff_pct > 10 ? '#dc2626' : '#16a34a';
  }
  if (storeLink) storeLink.href = v.store_url;
}

function initGift() {
  const btnDetect = el('btn-detect-editions');
  if (btnDetect) btnDetect.onclick = () => detectEditions();

  const btnCheck = el('btn-check-gift');
  if (btnCheck) btnCheck.onclick = () => checkGiftRisk();

  window.addEventListener('parallax:pagechange', event => {
    if (event.detail.id === 'gift') {
      loadGiftAccounts();
    }
  });

  if (location.hash === '#gift') {
    loadGiftAccounts();
  }
}

initGift();
loadGiftAccounts().catch(console.error);
