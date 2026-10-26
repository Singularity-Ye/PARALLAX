import {parallax} from './lib/api.js';
import {node} from './lib/pricing-view.js';

const box = document.getElementById('proxy-content');
const feedback = document.getElementById('proxy-feedback');

async function load() {
  try {
    const value = await parallax.proxyStatus();
    box.replaceChildren();

    // 1. 概览横幅 (Summary Bar)
    const summary = node('div');
    summary.className = 'proxy-summary';
    const isExplicit = value.mode === 'explicit_routes';
    
    const summaryLeft = node('div');
    const modeBadge = node('span', `模式：${isExplicit ? '显式平台路由' : '沿用系统环境代理'}`);
    modeBadge.className = `badge ${isExplicit ? 'ok' : 'muted'}`;
    const desc = node('p', `节点 ${value.nodes.length} 个 · 进程固定出口（不自动轮换） · 修改本地配置后需重启服务。`);
    desc.className = 'hint';
    desc.style.margin = '4px 0 0';
    summaryLeft.append(modeBadge, desc);

    const summaryRight = node('div');
    summaryRight.className = 'proxy-meta';
    summaryRight.textContent = isExplicit ? '未配置的平台默认直连' : '所有网络请求沿用系统环境';
    summary.append(summaryLeft, summaryRight);
    box.append(summary);

    // 2. 平台出口路由 (Platform Routes)
    const routesSection = node('section');
    routesSection.style.margin = '24px 0 30px';
    const routesHeader = node('div');
    routesHeader.className = 'section-title';
    routesHeader.append(node('h3', '平台出口路由'), node('span', 'EGRESS ROUTES'));
    routesSection.append(routesHeader);

    const routeEntries = Object.entries(value.routes || {});
    if (!routeEntries.length) {
      const notice = node('div');
      notice.className = 'notice';
      notice.textContent = isExplicit
        ? '当前未定义显式平台映射路由。所有请求均以直连方式访问目标平台。'
        : '当前运行于系统环境代理模式，未指定平台专属出口。';
      routesSection.append(notice);
    } else {
      const routeGrid = node('div');
      routeGrid.className = 'proxy-grid';
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
        meta.textContent = `该平台的固定出站通道为：${target}`;

        card.append(head, meta);
        routeGrid.append(card);
      }
      routesSection.append(routeGrid);
    }
    box.append(routesSection);

    // 3. 代理节点连通探测 (Proxy Nodes)
    const nodesSection = node('section');
    nodesSection.style.margin = '20px 0';
    const nodesHeader = node('div');
    nodesHeader.className = 'section-title';
    nodesHeader.append(node('h3', '代理节点连通探测'), node('span', 'NODE PROBING'));
    nodesSection.append(nodesHeader);

    if (!value.nodes.length) {
      const empty = node('div');
      empty.className = 'empty';
      empty.append(
        node('h3', '尚未配置代理节点'),
        node('p', '当前未配置出口节点。请使用本地 JSON 配置文件及 PARALLAX_PROXY_CONFIG；格式见 docs/network.md。本系统绝不读取其它软件的代理账号。')
      );
      nodesSection.append(empty);
    } else {
      const nodeGrid = node('div');
      nodeGrid.className = 'proxy-grid';

      for (const id of value.nodes) {
        const card = node('div');
        card.className = 'proxy-card';

        const head = node('div');
        head.className = 'proxy-card-head';
        head.append(node('h3', `节点 ID: ${id}`));
        const statusBadge = node('span', '未测试');
        statusBadge.className = 'badge muted';
        head.append(statusBadge);

        const meta = node('div');
        meta.className = 'proxy-meta';
        meta.textContent = '测试通过该节点向公网发出底层网络握手，探测 HTTP 响应与网络延迟。';

        const resultBox = node('div');
        resultBox.className = 'proxy-result-box';
        resultBox.style.display = 'none';

        const btn = node('button', `测试节点 ${id}`);
        btn.type = 'button';
        btn.className = 'secondary';
        btn.onclick = async () => {
          btn.disabled = true;
          btn.textContent = '正在探测…';
          try {
            const r = await parallax.probeProxy(id);
            if (r.outcome === 'http_response') {
              statusBadge.textContent = `HTTP ${r.http_status}`;
              statusBadge.className = 'badge ok';
              resultBox.className = 'proxy-result-box ok';
              resultBox.style.display = 'block';
              resultBox.replaceChildren();
              resultBox.append(
                node('div', `✓ 收到 HTTP ${r.http_status} · 往返延迟 ${r.latency_ms} ms`),
                node('small', '⚠️ 底层连通成功仅证明该代理通道物理可达；不代表已登录目标平台或拥有访问权限。')
              );
            } else {
              statusBadge.textContent = '连接失败';
              statusBadge.className = 'badge fail';
              resultBox.className = 'proxy-result-box fail';
              resultBox.style.display = 'block';
              resultBox.replaceChildren();
              resultBox.append(
                node('div', `✗ 连接失败 · 往返延迟 ${r.latency_ms} ms`),
                node('small', '⚠️ 探测失败；系统保持固定路由，不会静默自动切换节点或直连重试。')
              );
            }
          } catch (e) {
            statusBadge.textContent = '探测异常';
            statusBadge.className = 'badge fail';
            resultBox.className = 'proxy-result-box fail';
            resultBox.style.display = 'block';
            resultBox.textContent = `探测异常：${e.message}`;
          } finally {
            btn.disabled = false;
            btn.textContent = `重新测试 ${id}`;
          }
        };

        card.append(head, meta, btn, resultBox);
        nodeGrid.append(card);
      }
      nodesSection.append(nodeGrid);
    }
    box.append(nodesSection);

    feedback.textContent = '已读取进程当前网络与代理配置。';
  } catch (e) {
    feedback.textContent = `读取失败：${e.message}；原结果保留。`;
  }
}

document.getElementById('proxy-refresh').onclick = load;
load();
