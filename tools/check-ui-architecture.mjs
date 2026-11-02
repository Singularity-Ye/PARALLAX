// Isolated local browser regression: synthetic API only; never opens the user's profile.
import assert from 'node:assert/strict';
import {createServer} from 'node:http';
import {readFile} from 'node:fs/promises';
import {resolve,dirname,extname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
const require=createRequire(import.meta.url);
const {chromium}=require(process.env.PARALLAX_PLAYWRIGHT_MODULE||'playwright');
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../web');
const unknown={value:null,reason:'合成测试：没有市场证据'};
const row={row_id:'pending:a',lot_id:'lot-a',purchased_at:1,item_key:'<unsafe> 测试饰品',asset_id:null,quantity:2,stage:'pending_receipt',purchase_cost:{minor:667,currency:'CNY'},purchase_market_price:unknown,current_market_price:unknown,sale_net_wallet:unknown,actual_discount_bps:unknown,cash_profit:unknown,market_change:unknown};
let fail=false;const writes=[];
const server=createServer(async(req,res)=>{
 try {
 const url=new URL(req.url,'http://localhost');res.setHeader('Cache-Control','no-store');
 if(url.pathname.startsWith('/api/')){
  res.setHeader('Content-Type','application/json');let body={items:[]};
  if(req.method==='POST'){let text='';for await(const chunk of req)text+=chunk;writes.push(JSON.parse(text));if(writes.length===1){res.statusCode=500;body={error:'合成响应丢失，重试原请求'};}else body={replayed:true};}
  else if(url.pathname==='/api/accounts')body={items:[{id:'a',label:'测试 A'},{id:'b',label:'测试 B'}]};
  else if(url.pathname.endsWith('/ledger/summary'))body={counts:{held_units:0,listed_units:0,pending_units:2,sold_units:1},open_purchase_cost:{minor:667,currency:'CNY'},total_cash_profit:unknown,actual_discount_bps:unknown};
  else if(url.pathname.endsWith('/ledger/rows')){
   if(fail){res.statusCode=500;body={error:'合成读取失败'};}
   else {const offset=Number(url.searchParams.get('offset'));body={offset,total:51,next_offset:offset?null:50,items:offset?[{...row,row_id:'sold:a',stage:'sold',asset_id:'123',quantity:1,sale_net_wallet:{value:{minor:100,currency:'UAH'}}}]:[row]};}
  }
  res.end(JSON.stringify(body));return;
 }
 const path=resolve(root,'.'+(url.pathname==='/'?'/index.html':url.pathname));if(!path.startsWith(root+'/')&&!path.startsWith(root+'\\'))throw Error('invalid path');
 let content=await readFile(path);if(url.pathname==='/')content=Buffer.from(content.toString().replace(/<script\b[^>]*>[\s\S]*?<\/script>/g,'')+'<script type="module" src="/ledger.js"></script>');
 res.setHeader('Content-Type',extname(path)==='.js'?'text/javascript':extname(path)==='.css'?'text/css':'text/html');res.end(content);
 }catch(e){res.statusCode=404;res.end('not found');}
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));const base=`http://127.0.0.1:${server.address().port}`;
let browser;
try {
 browser=await chromium.launch({channel:'msedge',headless:true});const page=await browser.newPage({viewport:{width:1440,height:1000}});const errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.dismiss());
 await page.goto(base);await page.evaluate(()=>document.querySelectorAll('.page').forEach(p=>p.hidden=p.id!=='page-purchases'));
 await page.waitForFunction(()=>document.querySelector('#purchases-tbody').textContent.includes('测试饰品'));
 assert.equal(await page.locator('#purchases-tbody script').count(),0);assert.equal(await page.locator('#purchases-tbody td').nth(6).textContent(),'未知');
 await page.locator('#ledger-page-next').click();await page.waitForFunction(()=>document.querySelector('#purchases-tbody').textContent.includes('已登记成交'));
 assert.match(await page.locator('#purchases-tbody').textContent(),/已登记成交/);
 fail=true;await page.locator('#ledger-refresh').click();await page.waitForFunction(()=>document.querySelector('#ledger-feedback').textContent.includes('刷新未完成'));
 assert.match(await page.locator('#purchases-tbody').textContent(),/UAH|₴/);
 await page.locator('#ledger-account').selectOption('b');await page.waitForFunction(()=>document.querySelector('#ledger-feedback').textContent.includes('刷新未完成'));
 assert.equal(await page.locator('#summary-total-cost').textContent(),'未知');assert.doesNotMatch(await page.locator('#purchases-tbody').textContent(),/测试饰品/);
 fail=false;await page.locator('#add-purchase-name').fill('测试饰品');await page.locator('#add-purchase-price').fill('3.35');await page.locator('#add-purchase-quantity').fill('3');
 await page.locator('#btn-add-purchase').click();await page.waitForFunction(()=>!document.querySelector('#btn-add-purchase').disabled);
 assert.equal(await page.locator('#add-purchase-name').inputValue(),'测试饰品');
 await page.locator('#btn-add-purchase').click();await page.waitForFunction(()=>document.querySelector('#add-purchase-name').value==='');
 assert.equal(writes.length,2);assert.equal(writes[0].client_key,writes[1].client_key);assert.equal(writes[1].cash_cost_cents,1005);assert.deepEqual(writes[0],writes[1]);assert.deepEqual(errors,[]);
 // Native browser event ordering: short / intermediate / long / keyboard all submit once.
 await page.setContent(`<link rel="stylesheet" href="${base}/style.css"><form><button id="start" type="submit">扫描</button></form><a class="launch-card" href="#">测试卡片</a>`);
 await page.evaluate(async(base)=>{const {initMotion}=await import(base+'/lib/motion.js');const {initHoldButton}=await import(base+'/lib/hold-button.js');const {initMagnet}=await import(base+'/lib/magnet.js');const {initPageCurl}=await import(base+'/lib/page-curl.js');const {initClickSpark}=await import(base+'/lib/click-spark.js');const {initCardFx}=await import(base+'/lib/card-fx.js');window.stopMotion=initMotion([initHoldButton,initMagnet,initPageCurl,initClickSpark,initCardFx]);window.submits=0;document.querySelector('form').onsubmit=e=>{e.preventDefault();window.submits++;};},base);
 for(const delay of [100,300,600])await page.locator('#start').click({delay});await page.locator('#start').focus();await page.keyboard.press('Enter');assert.equal(await page.evaluate(()=>window.submits),4);
 await page.emulateMedia({reducedMotion:'reduce'});await page.waitForFunction(()=>!document.querySelector('.hold-fill,.page-curl-svg,#parallax-click-spark'));assert.equal(await page.locator('#start').evaluate(el=>el.style.translate),'');await page.locator('#start').click();assert.equal(await page.evaluate(()=>window.submits),5);
 await page.emulateMedia({reducedMotion:'no-preference'});await page.waitForFunction(()=>document.querySelectorAll('.hold-fill').length===1);assert.equal(await page.locator('.page-curl-svg').count(),1);assert.equal(await page.locator('#parallax-click-spark').count(),1);await page.evaluate(()=>window.dispatchEvent(new CustomEvent('parallax:pagechange')));assert.equal(await page.locator('.hold-fill').count(),1);assert.equal(await page.locator('.page-curl-svg').count(),1);assert.equal(await page.locator('#parallax-click-spark').count(),1);assert.deepEqual(errors,[]);
 console.log('Browser regression passed: ledger projection, failure preservation, account isolation, idempotent retry, native activation and live reduced-motion changes.');
}finally{await browser?.close();await new Promise(r=>server.close(r));}

