import {node} from './pricing-view.js';
export const decisionLabels={pass:'证据满足',fail:'存在阻断',unknown:'仍有未知'};
export const assetStageLabels={held:'持有',listed:'已挂',sold:'已售'};
export function decisionList(checks,names={}) {
  const list=document.createDocumentFragment();
  for(const c of checks){
    const details=node('details');details.append(node('summary',`${decisionLabels[c.status]} · ${names[c.module]??c.module}`),node('p',c.reason));
    if(c.observed!==null)details.append(node('pre',JSON.stringify(c.observed,null,2)));list.append(details);
  }
  return list;
}
