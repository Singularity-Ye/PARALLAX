import {parallax} from './api.js';
import {node} from './pricing-view.js';
export function strategyTransfer(container,getSelected,isBusy,onImported){
 const title=node('h3','策略备份与导入'),hint=node('p','导出当前选中的已保存版本；未保存编辑不包含在内。导入创建新策略v1，不覆盖原策略，不自动激活。仅支持Parallax策略包，原项目格式尚未转换。');
 const text=node('textarea');text.rows=10;text.maxLength=16384;text.setAttribute('aria-label','策略包 JSON');
 const file=node('input');file.type='file';file.accept='.json,application/json';file.setAttribute('aria-label','读取策略包文件');
 const exportButton=node('button','导出已保存版本'),importButton=node('button','导入为新策略'),download=node('a','下载已导出的策略包'),feedback=node('p');
 exportButton.type=importButton.type='button';feedback.setAttribute('role','status');download.hidden=true;let working=false,url=null;
 function lock(value){working=value;exportButton.disabled=importButton.disabled=file.disabled=value;}
 file.onchange=async()=>{if(working||isBusy())return;const selected=file.files[0];if(!selected)return;
  if(selected.size>16384){feedback.textContent='文件超过16KiB，未读取；原输入保留。';return;}lock(true);
  try{text.value=await selected.text();feedback.textContent='文件已载入，尚未保存。检查内容后点击导入。';}catch(e){feedback.textContent=`读取失败：${e.message}；原输入保留。`;}finally{lock(false);}};
 exportButton.onclick=async()=>{if(working||isBusy())return;const version=getSelected();if(!version){feedback.textContent='先选择一个已保存策略版本。';return;}lock(true);
  try{const data=await parallax.exportStrategy(version.strategy_id,version.revision);const json=JSON.stringify(data,null,2);text.value=json;
   if(url)URL.revokeObjectURL(url);url=URL.createObjectURL(new Blob([json],{type:'application/json'}));download.href=url;download.download=`parallax-strategy-${version.strategy_id}-v${version.revision}.json`;download.hidden=false;
   feedback.textContent=`已导出保存版本v${version.revision}；只有规则参数，不含账户或研究数据。`;
  }catch(e){feedback.textContent=`导出失败：${e.message}；原输入保留。`;}finally{lock(false);}};
 importButton.onclick=async()=>{if(working||isBusy())return;if(new TextEncoder().encode(text.value).length>16384){feedback.textContent='策略包超过16KiB，未导入。';return;}lock(true);
  try{const data=JSON.parse(text.value),saved=await parallax.importStrategy(data);feedback.textContent=`已创建新策略“${saved.draft.name}”v1（${saved.strategy_id}）；当前编辑和旧版本保留。`;
   try{await onImported(saved);}catch(e){feedback.textContent+=` 列表刷新失败：${e.message}；策略已保存，请刷新记录，不必重复导入。`;}
  }catch(e){feedback.textContent=`导入未确认：${e.message}；输入保留。网络结果不明时先刷新策略列表核对，避免重复导入。`;}finally{lock(false);}};
 container.append(title,hint,file,text,exportButton,download,importButton,feedback);
}
