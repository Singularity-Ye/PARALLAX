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
export const shelltrade = {
  health: () => request('/api/health'),
  scans: () => request('/api/scans'),
  scan: id => request(`/api/scans/${encodeURIComponent(id)}`),
  startScan: value => post('/api/scans', value),
  cancelScan: id => post(`/api/scans/${encodeURIComponent(id)}/cancel`, {}),
  plans: () => request('/api/plans'),
  plan: id => request(`/api/plans/${encodeURIComponent(id)}`),
  createPlan: value => post('/api/plans', value),
};
