import {parallax} from './lib/api.js';
import {node} from './lib/pricing-view.js';
const el=id=>document.getElementById(`catalog-${id}`);
let busy=false,page=null;
function ready(){el('search').disabled=busy;el('next').disabled=busy||page?.next_start==null;el('query').disabled=busy;}
async function search(start){
  if(busy)return;busy=true;ready();
  const query=start===0?el('query').value.trim():page.query;
  el('feedback').textContent='正在读取Steam市场的一页检索结果…';
  try{
    const result=await parallax.searchCatalog({query,start});page=result;
    const content=document.createDocumentFragment();
    for(const item of result.items){
      const card=node('article');card.className='candidate';
      card.append(node('h3',item.name),node('p',item.market_hash_name),node('p',`在售挂牌数：${item.sell_listings??'未知'}；不是日成交量，也不是可按某价成交的数量。`));
      const button=node('button','带入历史研究');button.type='button';button.className='secondary';
      button.onclick=()=>window.dispatchEvent(new CustomEvent('parallax:catalog-history',{detail:{item_key:item.market_hash_name}}));
      card.append(button);content.append(card);
    }
    if(!result.items.length)content.append(node('p','平台确认本页没有匹配商品。'));
    el('results').replaceChildren(content);
    el('feedback').textContent=`来源：Steam市场关键词检索 · ${new Date(result.observed_at*1000).toLocaleString()} · 起点 ${result.start} · 本页 ${result.received} 项 · 平台当次总数 ${result.upstream_total}（可能变化）。${result.next_start==null?'本次已无可继续的分页或达到起点上限；不代表收集了全部市场。':'只展示当前页，未自动遍历。'}`;
  }catch(e){el('feedback').textContent=`读取未完成：${e.message}；输入及上一页结果保留。`;}
  finally{busy=false;ready();}
}
el('form').onsubmit=e=>{e.preventDefault();search(0);};el('next').onclick=()=>{if(page?.next_start!=null)search(page.next_start);};ready();
