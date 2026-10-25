import {parallax} from './api.js';
import {node,nativeMoney} from './pricing-view.js';
const status={pass:'满足',fail:'未满足',unknown:'未知'};
const percent=v=>v==null?'未知':`${(v/100).toFixed(2)}%`;
const day=d=>d==null?'未知':new Date(d*86400000).toISOString().slice(0,10);
export function candidateComparison(report,isCurrent,modules){
 const section=node('section'),feedback=node('p','正在读取保存证据的横向比较…'),content=node('div');
 section.append(node('h3','所选商品横向比较'),feedback,content);feedback.setAttribute('role','status');
 const retry=node('button','重新读取比较');retry.type='button';section.append(retry);
 async function load(){retry.disabled=true;try{
  const comparison=await parallax.holdingComparison(report.account_id,report.id);if(!isCurrent())return;
  const table=node('table'),head=node('tr');
  for(const name of ['商品 / 口径组','原判断','原币参考日价','覆盖 / 缺失日','原始 / 清洗CV','七日斜率/日','历史P10','未满足 / 未知规则','采集时间'])head.append(node('th',name));table.append(head);
  const names=keys=>keys.map(k=>modules[k]??k).join('、')||'无';
  for(const row of comparison.rows){const tr=node('tr');for(const value of [
   `${row.item_key} / ${row.comparison_group==null?'口径未知':`组${row.comparison_group}`}`,
   status[row.status],`${row.reference_price?nativeMoney(row.reference_price.minor,row.reference_price.currency):'未知'} · ${day(row.reference_utc_day)}`,
   `${row.usable_days??'未知'} / ${row.missing_days??'未知'} (${day(row.from_utc_day)} 至 ${day(row.until_utc_day_exclusive)} 前)`,
   `${percent(row.raw_cv_bps)} / ${percent(row.clean_cv_bps)}`,percent(row.trend_bps_per_day),percent(row.calendar_p10_bps),
   `${names(row.failed_rules)} / ${names(row.unknown_rules)}`,row.observed_at==null?'未知':new Date(row.observed_at*1000).toLocaleString()
  ])tr.append(node('td',value));table.append(tr);}
  const wrap=node('div');wrap.style.overflowX='auto';wrap.append(table);content.replaceChildren(wrap);
  for(const text of comparison.limitations)content.append(node('p',text,'hint'));
  feedback.textContent=`${comparison.rows.length}件商品 · 沿用原策略v${comparison.revision} · 按原研究顺序，无综合买入排名。`;
 }catch(e){if(isCurrent())feedback.textContent=`比较暂未读取：${e.message}。原研究与已显示结果保留。`;}finally{retry.disabled=false;}}
 retry.onclick=load;load();return section;
}
