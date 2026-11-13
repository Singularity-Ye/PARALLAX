export function isDemoKey(key) {
  if (!key) return true;
  const s = String(key).trim();
  if (!s) return true;
  const lower = s.toLowerCase();
  return lower.startsWith('demo-') || s.startsWith('演示·') || lower.includes('fixture');
}

export function inferItemType(name = '') {
  const s = String(name);
  if (/Package|Souvenir|Case|Capsule|胶囊|纪念包|武器箱/i.test(s)) return 'case';
  if (/★|Knife|Karambit|Bayonet|Gloves|蝴蝶刀|爪子刀|刺刀/i.test(s)) return 'knife';
  if (/Sticker|Patch|Music Kit|Pin|印花|臂章/i.test(s)) return 'sticker';
  return 'skin';
}

export function getItemSvgPlaceholder(nameOrType = 'case') {
  const type = ['case', 'knife', 'sticker', 'skin'].includes(nameOrType)
    ? nameOrType
    : inferItemType(nameOrType);
  const stroke = '%235f6b54';
  const fillBg = '%23f4f6ed';
  const fillSoft = '%23e2ead6';
  const icons = {
    case: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="10" fill="${fillBg}"/><rect x="14" y="20" width="36" height="26" rx="4" fill="${fillSoft}" stroke="${stroke}" stroke-width="2.5"/><path d="M14 28h36M28 28v6h8v-6" stroke="${stroke}" stroke-width="2.5" fill="none"/><path d="M24 20v-4a2 2 0 0 1 2-2h12a2 2 0 0 1 2 2v4" stroke="${stroke}" stroke-width="2.5" fill="none"/></svg>`,
    knife: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="10" fill="${fillBg}"/><path d="M16 46l18-18 12-10-4 14-18 16-8-2z" fill="${fillSoft}" stroke="${stroke}" stroke-width="2.5" stroke-linejoin="round"/></svg>`,
    sticker: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="10" fill="${fillBg}"/><circle cx="32" cy="32" r="15" fill="${fillSoft}" stroke="${stroke}" stroke-width="2.5"/><path d="M25 32l5 5 10-10" stroke="${stroke}" stroke-width="2.5" fill="none" stroke-linecap="round" stroke-linejoin="round"/></svg>`,
    skin: `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="10" fill="${fillBg}"/><path d="M12 36l8-10h24l8 6-6 6H24l-4 8h-6l2-10h-4z" fill="${fillSoft}" stroke="${stroke}" stroke-width="2.5" stroke-linejoin="round"/></svg>`,
  };
  return `data:image/svg+xml;utf8,${icons[type] || icons.skin}`;
}

export function iconCacheUrl(itemKey, rawUrl = null) {
  let path = `/api/assets/icon?name=${encodeURIComponent(itemKey)}`;
  if (rawUrl && typeof rawUrl === 'string' && rawUrl.trim()) {
    path += `&url=${encodeURIComponent(rawUrl.trim())}`;
  }
  return path;
}

export function createItemThumb(itemKey, rawUrl = null, size = 'md', extraClass = '') {
  const img = document.createElement('img');
  img.className = `item-icon-thumb item-icon-${size} ${extraClass}`.trim();
  img.alt = itemKey || 'item';
  img.loading = 'lazy';
  const fallback = getItemSvgPlaceholder(itemKey);
  if (!itemKey || isDemoKey(itemKey)) {
    img.src = fallback;
    return img;
  }
  img.src = iconCacheUrl(itemKey, rawUrl);
  img.onerror = () => {
    img.onerror = null;
    img.src = fallback;
  };
  return img;
}
