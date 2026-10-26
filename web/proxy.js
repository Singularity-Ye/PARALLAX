import {parallax} from './lib/api.js';
import {node} from './lib/pricing-view.js';
const box=document.getElementById('proxy-content'),feedback=document.getElementById('proxy-feedback');
async function load(){try{const value=await parallax.proxyStatus();box.replaceChildren(node('p',`当前模式：${value.mode==='explicit_routes'?'显式平台路由（未配置的平台直连）':'沿用系统环境代理'}。节点 ${value.nodes.length} 个；不自动轮换；修改本地配置后需重启。`));
 for(const [platform,target]of Object.entries(value.routes))box.append(node('p',`${platform} → ${target}`));
 for(const id of value.nodes){const section=node('section'),button=node('button',`测试节点 ${id}`),result=node('p');button.type='button';button.onclick=async()=>{button.disabled=true;try{const r=await parallax.probeProxy(id);result.textContent=r.outcome==='http_response'?`收到HTTP ${r.http_status} · ${r.latency_ms}ms；不代表登录/访问权限已验证。`:`连接失败 · ${r.latency_ms}ms；没有切换路由或直连重试。`;}catch(e){result.textContent=e.message;}finally{button.disabled=false;}};section.append(button,result);box.append(section);}
 if(!value.nodes.length)box.append(node('p','尚未配置节点。请使用本地JSON配置及PARALLAX_PROXY_CONFIG；格式见docs/network.md。不会读取其它软件的代理账号。'));feedback.textContent='已读取进程当前配置。';
 }catch(e){feedback.textContent=`读取失败：${e.message}；原结果保留。`;}}
document.getElementById('proxy-refresh').onclick=load;load();
