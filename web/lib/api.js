// UI-independent client. A replacement frontend can reuse this module as-is.
export class ApiError extends Error {
  constructor(message, status) { super(message); this.name = 'ApiError'; this.status = status; }
}
async function request(path, options = {}) {
  const response = await fetch(path, {...options, headers: {'Content-Type':'application/json', ...options.headers}});
  const text = await response.text();
  let data;
  try { data = JSON.parse(text); }
  catch { throw new ApiError(`服务返回无法读取的响应（HTTP ${response.status}）`, response.status); }
  if (!response.ok) throw new ApiError(data.error || `请求失败（HTTP ${response.status}）`, response.status);
  return data;
}
const post = (path, value) => request(path, {method:'POST', body:JSON.stringify(value)});
export const parallax = {
  purchaseLots: account => request(`/api/accounts/${encodeURIComponent(account)}/purchase-lots`),
  createPurchaseLot: (account,value) => post(`/api/accounts/${encodeURIComponent(account)}/purchase-lots`,value),
  purchaseLot: (account,id) => request(`/api/accounts/${encodeURIComponent(account)}/purchase-lots/${encodeURIComponent(id)}`),
  lotEvents: (account,id) => request(`/api/accounts/${encodeURIComponent(account)}/purchase-lots/${encodeURIComponent(id)}/events`),
  appendLotEvent: (account,id,value) => post(`/api/accounts/${encodeURIComponent(account)}/purchase-lots/${encodeURIComponent(id)}/events`,value),
  holdingsPreview: (account,value) => post(`/api/accounts/${encodeURIComponent(account)}/holdings-preview`,value),
  saleResearches: account => request(`/api/accounts/${encodeURIComponent(account)}/sale-research`),
  saleResearch: (account,id) => request(`/api/accounts/${encodeURIComponent(account)}/sale-research/${encodeURIComponent(id)}`),
  createSaleResearch: (account,value) => post(`/api/accounts/${encodeURIComponent(account)}/sale-research`,value),
  preflights: () => request('/api/preflights'),
  preflight: id => request(`/api/preflights/${encodeURIComponent(id)}`),
  createPreflight: value => post('/api/preflights',value),
  strategies: () => request('/api/strategies'),
  createStrategy: value => post('/api/strategies',value),
  strategyVersions: id => request(`/api/strategies/${encodeURIComponent(id)}/versions`),
  strategyVersion: (id,revision) => request(`/api/strategies/${encodeURIComponent(id)}/versions/${revision}`),
  reviseStrategy: (id,value) => post(`/api/strategies/${encodeURIComponent(id)}/versions`,value),
  evaluateStrategy: value => post('/api/strategy-evaluations',value),
  strategyEvaluations: () => request('/api/strategy-evaluations'),
  strategyEvaluation: id => request(`/api/strategy-evaluations/${encodeURIComponent(id)}`),
  priceHistories: account => request(`/api/accounts/${encodeURIComponent(account)}/price-histories`),
  priceHistory: (account,id) => request(`/api/accounts/${encodeURIComponent(account)}/price-histories/${encodeURIComponent(id)}`),
  fetchPriceHistory: (account,value) => post(`/api/accounts/${encodeURIComponent(account)}/price-histories`,value),
  inventories: account => request(`/api/accounts/${encodeURIComponent(account)}/inventories`),
  inventory: (account,id) => request(`/api/accounts/${encodeURIComponent(account)}/inventories/${encodeURIComponent(id)}`),
  fetchInventory: (account,value) => post(`/api/accounts/${encodeURIComponent(account)}/inventories`,value),
  buffQuotes: account => request(`/api/accounts/${encodeURIComponent(account)}/buff-quotes`),
  buffQuote: (account,id) => request(`/api/accounts/${encodeURIComponent(account)}/buff-quotes/${encodeURIComponent(id)}`),
  fetchBuffQuote: (account,value) => post(`/api/accounts/${encodeURIComponent(account)}/buff-quotes`,value),
  orderBooks: () => request('/api/steam/order-books'),
  orderBook: id => request(`/api/steam/order-books/${encodeURIComponent(id)}`),
  fetchOrderBook: value => post('/api/steam/order-books', value),
  listingPricePreview: (id,value) => post(`/api/steam/order-books/${encodeURIComponent(id)}/listing-preview`,value),
  orderBookValuation: (id,value={}) => post(`/api/steam/order-books/${encodeURIComponent(id)}/valuation`,value),
  health: () => request('/api/health'),
  accounts: () => request('/api/accounts'),
  createAccount: value => post('/api/accounts', value),
  sessions: id => request(`/api/accounts/${encodeURIComponent(id)}/sessions`),
  importSession: (id, platform, value) => post(`/api/accounts/${encodeURIComponent(id)}/sessions/${platform}`, value),
  verifySession: (id, platform) => post(`/api/accounts/${encodeURIComponent(id)}/sessions/${platform}/verify`, {}),
  revokeSession: (id, platform) => request(`/api/accounts/${encodeURIComponent(id)}/sessions/${platform}`, {method:'DELETE'}),
  browserLogin: () => request('/api/browser-logins'),
  startBrowserLogin: (id, platform) => post(`/api/accounts/${encodeURIComponent(id)}/sessions/${platform}/browser`, {}),
  finishBrowserLogin: id => post(`/api/browser-logins/${encodeURIComponent(id)}/finish`, {}),
  cancelBrowserLogin: id => request(`/api/browser-logins/${encodeURIComponent(id)}`, {method:'DELETE'}),
  scans: () => request('/api/scans'),
  scan: id => request(`/api/scans/${encodeURIComponent(id)}`),
  startScan: value => post('/api/scans', value),
  cancelScan: id => post(`/api/scans/${encodeURIComponent(id)}/cancel`, {}),
  plans: () => request('/api/plans'),
  plan: id => request(`/api/plans/${encodeURIComponent(id)}`),
  createPlan: value => post('/api/plans', value),
};
