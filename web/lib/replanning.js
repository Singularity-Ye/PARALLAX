import {parallax} from './api.js';
import {node} from './pricing-view.js';
export function researchReplanning(report){
 const section=node('section');section.append(node('h3','按研究结果重新规划预算'),node('p','仅选择本份保存研究中满足规则的商品。沿用原扫描与原报价引用，不自动刷新；按当前时效重新分配。旧通过结果不适用于新数量，新计划必须重新研究。'));
 const form=node('form'),choices=[];
 for(const item of report.items){const label=node('label'),check=node('input');check.type='checkbox';check.value=item.item_key;check.disabled=item.status!=='pass';label.append(check,document.createTextNode(`${item.item_key} · ${item.status==='pass'?'可选择':item.status==='fail'?'未满足，不纳入':'未知，不纳入'}`));form.append(label);if(item.status==='pass')choices.push(check);}
 const field=(title,value,min,max,step)=>{const label=node('label',title),input=node('input');input.type='number';input.min=min;input.max=max;input.step=step;input.required=true;input.value=value;label.append(input);form.append(label);return input;};
 const budget=field('新预算 / 人民币元',(report.plan.request.budget_cents/100).toFixed(2),'0.01','1000000','0.01');
 const units=field('每商品硬数量上限',report.plan.request.max_units_per_item,'1','100','1');
 const positions=field('商品种类上限',report.plan.request.max_positions,'1','100','1');
 const submit=node('button','保存重新规划的预算');submit.type='submit';submit.disabled=!choices.length;const feedback=node('p');feedback.setAttribute('role','status');const output=node('div');
 form.append(submit);section.append(form,feedback,output);
 if(!choices.length)feedback.textContent='本研究没有满足规则的商品，不能据此重新规划；请补齐证据或另做研究。';
 let busy=false;
 form.onsubmit=async event=>{event.preventDefault();if(busy)return;const selected=choices.filter(c=>c.checked).map(c=>c.value);if(!selected.length){feedback.textContent='请明确选择至少一个满足规则的商品。';return;}busy=true;for(const control of form.elements)control.disabled=true;
 try{const request={...report.plan.request,selected_item_keys:selected,budget_cents:Math.round(Number(budget.value)*100),max_units_per_item:Number(units.value),max_positions:Number(positions.value)};
 const plan=await parallax.replan({evaluation_id:report.id,request});feedback.textContent=`新预算已保存：${plan.id} · ${plan.lines.length}个分配商品；尚未对新数量完成策略复核。`;
 const open=node('button','查看新预算与跳过原因');open.type='button';open.onclick=()=>window.dispatchEvent(new CustomEvent('parallax:plan-replanned',{detail:plan}));output.replaceChildren(open);
 }catch(e){feedback.textContent=`重新规划未确认：${e.message}；选择与原结果保留。网络结果未知时先查看预算历史，避免重复保存。`;}
 finally{busy=false;for(const control of form.elements)control.disabled=false;for(const check of form.querySelectorAll('input[type=checkbox]'))check.disabled=!choices.includes(check);}};
 return section;
}
