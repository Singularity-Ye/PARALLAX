import {node} from './pricing-view.js';
export function populateSelect(select,items,placeholder,label,value=item=>item.id) {
  const previous=select.value;
  const options=items.map(item=>{const n=node('option',label(item));n.value=String(value(item));return n;});
  const empty=node('option',placeholder);empty.value='';select.replaceChildren(empty,...options);
  if(items.some(item=>String(value(item))===previous))select.value=previous;
}
export function localTime(at=Date.now()) {const d=new Date(at);return new Date(at-d.getTimezoneOffset()*60000).toISOString().slice(0,19);}
export function unixTime(field) {const t=new Date(field.value).getTime();if(!Number.isFinite(t))throw new Error('请填写有效的本地日期时间');return Math.floor(t/1000);}
// Retrying identical input reuses its key, including after an uncertain HTTP response.
export function retryableCommand(){let previous=null;return value=>{const signature=JSON.stringify(value);if(previous?.signature!==signature)previous={signature,key:crypto.randomUUID()};return {...value,client_key:previous.key};};}
