import {groups,pages,resolvePage} from './lib/pages.js';
import {parallax} from './lib/api.js';
const el=id=>document.getElementById(id);
const node=(tag,text)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
const iconPaths={
  grid:'M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z',
  search:'M20 20l-5-5 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0',
  plan:'M5 3h14v18H5z M8 8h8 M8 12h8 M8 16h4',
  box:'M3 7l9-4 9 4v11l-9 4-9-4z M3 7l9 5 9-5 M12 12v10',
  layers:'M3 7l9-5 9 5-9 5z M3 12l9 5 9-5 M3 17l9 5 9-5',
  journal:'M5 3h14v18H5z M8 7h8 M8 11h8 M8 15h5',
  chart:'M4 3v17h17 M8 16v-5 M13 16V7 M18 16V4',
  tag:'M3 3h9l9 9-9 9-9-9z M7 7h.01',
  user:'M20 21v-3a5 5 0 0 0-5-5H9a5 5 0 0 0-5 5v3 M16 6a4 4 0 1 1-8 0 4 4 0 0 1 8 0',
  shield:'M12 2l9 4v6c0 5-9 10-9 10S3 17 3 12V6z M8 12l3 3 5-6',
  sliders:'M5 3v7 M5 14v7 M12 3v11 M12 18v3 M19 3v3 M19 10v11 M2 10h6 M9 18h6 M16 6h6',
  globe:'M22 12A10 10 0 1 1 2 12a10 10 0 0 1 20 0 M2 12h20 M12 2c-6 5-6 15 0 20 M12 2c6 5 6 15 0 20',
  terminal:'M4 5l6 6-6 6 M12 19h8',
  gift:'M3 8h18v5H3z M5 13v9h14v-9 M12 8v14 M12 8H7a3 3 0 1 1 3-3z M12 8h5a3 3 0 1 0-3-3z',
};
function icon(name) {
  const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');
  for(const [k,v] of Object.entries({viewBox:'0 0 24 24',fill:'none',stroke:'currentColor','stroke-width':'1.7','stroke-linecap':'round','stroke-linejoin':'round','aria-hidden':'true'}))svg.setAttribute(k,v);
  const path=document.createElementNS(svg.namespaceURI,'path');path.setAttribute('d',iconPaths[name]??iconPaths.grid);svg.append(path);return svg;
}
for(const [index,label] of groups.entries()) {
  const group=node('div');group.className='nav-group';group.append(node('p',label));
  for(const page of pages.filter(p=>p.group===index)) {
    const link=node('a');link.href=`#${page.id}`;link.dataset.route=page.id;
    if(page.child)link.classList.add('nav-child');
    link.append(icon(page.icon),node('span',page.title));
    if(page.pending){const badge=node('small','待接入');badge.className='nav-pending';link.append(badge);}
    group.append(link);
  }
  el('page-navigation').append(group);
}
for(const page of pages.filter(p=>p.pending)) {
  const section=node('section');section.className='page';section.dataset.page=page.id;section.id=`page-${page.id}`;section.hidden=true;
  const card=node('div');card.className='card capability-card';
  const stamp=node('span','待迁移');stamp.className='label-stamp neutral';
  card.append(stamp,node('h2',`${page.title}正在重制。`),node('p','保留原项目的功能位置。这里尚未连接真实业务，当前不会发起相关操作。'));
  const list=node('ul');list.className='capability-list';for(const item of page.pending)list.append(node('li',item));card.append(list);
  const related=pages.find(p=>p.id===page.related);const link=node('a',`前往${related.title} →`);link.href=`#${related.id}`;link.className='text-link';card.append(link);section.append(card);el('app').insertBefore(section,document.querySelector('footer'));
}
const mobile=window.matchMedia('(max-width: 900px)');
let navOpen=false;
function setNav(open,{restoreFocus=false}={}) {
  navOpen=mobile.matches&&open;document.body.dataset.navOpen=String(navOpen);
  el('nav-toggle').setAttribute('aria-expanded',String(navOpen));el('nav-backdrop').hidden=!navOpen;
  el('sidebar').inert=mobile.matches&&!navOpen;
  if(navOpen)el('nav-close').focus();else if(restoreFocus&&mobile.matches)el('nav-toggle').focus();
}
el('nav-toggle').onclick=()=>setNav(!navOpen,{restoreFocus:true});
el('nav-close').onclick=el('nav-backdrop').onclick=()=>setNav(false,{restoreFocus:true});
mobile.addEventListener('change',()=>setNav(false));setNav(false);
document.addEventListener('keydown',event=>{
  if(!navOpen)return;
  if(event.key==='Escape'){event.preventDefault();setNav(false,{restoreFocus:true});}
  if(event.key==='Tab') {
    const focusable=[...el('sidebar').querySelectorAll('a,button')].filter(n=>!n.hidden&&n.getClientRects().length);
    const first=focusable[0],last=focusable.at(-1);
    if(event.shiftKey&&document.activeElement===first){event.preventDefault();last.focus();}
    else if(!event.shiftKey&&document.activeElement===last){event.preventDefault();first.focus();}
  }
});
let active=null;const scrollPositions=new Map();
function route(initial=false) {
  const page=resolvePage(location.hash);const id=page?.id??'not-found';
  if(active!==id){if(active)scrollPositions.set(active,window.scrollY);active=id;}
  for(const view of document.querySelectorAll('[data-page]'))view.hidden=view.dataset.page!==id;
  for(const link of document.querySelectorAll('[data-route]')) {
    const selected=link.dataset.route===id;link.classList.toggle('active',selected);
    if(selected)link.setAttribute('aria-current','page');else link.removeAttribute('aria-current');
  }
  el('page-heading').textContent=page?.title??'页面未找到';
  el('page-description').textContent=page?.description??'请从导航选择一个功能页面。';
  el('page-kicker').textContent=page?.kicker??'PAGE / NOT FOUND';
  el('page-breadcrumb').textContent=page?`${groups[page.group]} / ${page.title}`:'页面未找到';
  el('page-number').textContent=page?String(pages.indexOf(page)+1).padStart(2,'0'):'—';
  el('app').dataset.currentPage=id;document.title=`${page?.title??'页面未找到'} · Parallax`;
  setNav(false);window.scrollTo({top:scrollPositions.get(id)??0,behavior:'instant'});
  if(!initial)el('page-heading').focus({preventScroll:true});
  window.dispatchEvent(new CustomEvent('parallax:pagechange',{detail:{id}}));
}
window.addEventListener('hashchange',()=>route());route(true);
document.addEventListener('click',event=>{
  if(event.target.closest('[data-open-positions]'))el('manual-positions').open=true;
  if(event.target.closest('a[href="#page-heading"]')){event.preventDefault();el('page-heading').focus();}
  const link=event.target.closest('[data-route]');
  if(link&&resolvePage(location.hash)?.id===link.dataset.route){setNav(false);el('page-heading').focus({preventScroll:true});}
});
window.addEventListener('parallax:scan-selected',({detail:r})=>{
  el('dashboard-source').textContent=r.request.source==='demo'?'演示数据':'SteamDT 榜单';
  el('dashboard-scan-time').textContent=`${new Date(r.created_at*1000).toLocaleString()} · ${r.status==='completed'?'已完成':r.status==='running'?'扫描中':r.status}`;
  el('dashboard-selection').textContent=`${r.coverage.accepted} / ${r.coverage.examined}`;
});
parallax.health().then(health=>{
  el('dashboard-service').textContent='已连接';el('dashboard-service-note').textContent=health.capabilities.live_execution?'请核对执行状态':'自动交易未接入 · 仅研究与预览';
  el('sidebar-status').textContent=health.capabilities.live_execution?'请核对执行状态':'研究与预算预览';
  el('mode-badge').textContent=health.capabilities.live_execution?'执行状态需核对':'研究模式 · 自动交易未接入';
}).catch(()=>{el('dashboard-service').textContent='连接未确认';el('dashboard-service-note').textContent='请检查本地服务后刷新，已填内容保留';el('mode-badge').textContent='服务连接未确认';});
