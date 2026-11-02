import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
function source(file){return readFileSync(new URL('../../web/'+file,import.meta.url),'utf8').replace(/^import .*;\r?\n/gm,'').replace(/export (function|const)/g,'$1');}
function element(tag='div',text=''){
 const listeners=new Map(),props=new Map();
 return {tag,textContent:text,children:[],style:{setProperty:(k,v)=>props.set(k,v),removeProperty:k=>props.delete(k),getPropertyValue:k=>props.get(k)},dataset:{},className:'',disabled:false,isConnected:true,
 classList:{add(){},remove(){}},setAttribute(k,v){this[k]=v;},append(...items){this.children.push(...items);},prepend(n){this.children.unshift(n);n.parent=this;},remove(){this.parent?.children.splice(this.parent.children.indexOf(this),1);},replaceChildren(...items){this.children=items;},
 addEventListener(type,fn){if(!listeners.has(type))listeners.set(type,new Set());listeners.get(type).add(fn);},removeEventListener(type,fn){listeners.get(type)?.delete(fn);},emit(type,event={}){for(const fn of [...listeners.get(type)??[]])fn(event);},count(type){return listeners.get(type)?.size??0;}};
}
function motion(){let time=0,id=0;const frames=new Map(),win=element(),doc=element(),pref=element();pref.matches=false;doc.hidden=false;
 const context=vm.createContext({console,Math,Number,Set,Map,performance:{now:()=>time},requestAnimationFrame:fn=>{frames.set(++id,fn);return id;},cancelAnimationFrame:id=>frames.delete(id),window:win,document:doc,matchMedia:()=>pref});
 vm.runInContext(source('lib/motion.js'),context);return {context,frames,win,doc,pref,tick(now){time=now;const batch=[...frames.values()];frames.clear();batch.forEach(fn=>fn(now));}};
}
test('spring has consistent elapsed-time behavior at 60 and 144 Hz',()=>{
 const {context:c}=motion();function run(hz){const state={x:0,v:0};for(let i=0;i<hz/2;i++)c.springStep(state,14,1/hz,180,14);return state.x;}
 assert.ok(Math.abs(run(60)-run(144))<0.1);assert.ok(Math.abs(run(60)-14)<0.5);
});
test('lifecycle disposes listeners and frames on reduced motion, visibility and route changes',()=>{
 const h=motion();let mounts=0,disposals=0;h.context.effects=[()=>{mounts++;const s=h.context.motionScope();s.on(h.win,'pointermove',()=>{});s.frame(()=>assert.fail('disposed frame ran'));s.cleanup(()=>disposals++);return ()=>s.dispose();}];
 const dispose=vm.runInContext('initMotion(effects)',h.context);assert.equal(h.win.count('pointermove'),1);
 h.pref.matches=true;h.pref.emit('change');assert.equal(disposals,1);assert.equal(h.frames.size,0);assert.equal(h.win.count('pointermove'),0);
 h.pref.matches=false;h.pref.emit('change');h.doc.hidden=true;h.doc.emit('visibilitychange');assert.equal(h.frames.size,0);
 h.doc.hidden=false;h.doc.emit('visibilitychange');h.win.emit('parallax:pagechange');assert.equal(h.win.count('pointermove'),1);assert.equal(mounts,4);dispose();assert.equal(h.win.count('pointermove'),0);assert.equal(h.pref.count('change'),0);
});
test('short, medium, long and keyboard activation keep exactly one native click; decoration never submits',()=>{
 const h=motion(),btn=element('button');let clicks=0;
 btn.click=()=>assert.fail('effect called click');btn.form={requestSubmit:()=>assert.fail('effect submitted')};
 btn.addEventListener('click',()=>clicks++);h.doc.querySelectorAll=()=>[btn];h.doc.createElement=tag=>element(tag);
 vm.runInContext(source('lib/hold-button.js'),h.context);
 const stop=h.context.initHoldButton();
 for(const duration of [100,300,600]){btn.emit('pointerdown',{button:0});h.tick(duration+1000*clicks);btn.emit('pointerup');btn.emit('click');}
 btn.emit('click');assert.equal(clicks,4);stop();assert.equal(h.frames.size,0);assert.equal(btn.count('pointerdown'),0);assert.equal(btn.children.length,0);
});
test('ledger rows preserve quantity, native currency, zero and unknown without HTML injection',()=>{
 const c=vm.createContext({node:element,nativeMoney:(n,c)=>`${c} ${n}`,Date});vm.runInContext(source('lib/ledger-view.js'),c);
 const metric={value:null,reason:'no quote'},base={row_id:'pending:x',lot_id:'x',purchased_at:1,item_key:'<unsafe>',asset_id:null,quantity:2,stage:'pending_receipt',purchase_cost:{minor:667,currency:'CNY'},purchase_market_price:metric,current_market_price:metric,sale_net_wallet:metric,actual_discount_bps:metric,cash_profit:metric,market_change:metric};
 const body=element();c.renderLedgerTable(body,{items:[base,{...base,asset_id:'a',quantity:1,stage:'sold',sale_net_wallet:{value:{minor:0,currency:'UAH'}}}]});
 assert.equal(body.children[0].children.length,13);assert.equal(body.children[0].children[2].children[0].textContent,'<unsafe>');assert.equal(body.children[0].children[4].textContent,'CNY 667');assert.equal(body.children[0].children[6].textContent,'未知');assert.equal(body.children[0].children[6].title,'no quote');assert.equal(body.children[1].children[7].textContent,'UAH 0');assert.equal(body.children[0].children[3].textContent,'未绑定');
 const els=new Map();c.renderLedgerSummary(id=>{if(!els.has(id))els.set(id,element());return els.get(id);},{counts:{held_units:1,listed_units:1,pending_units:2,sold_units:1},open_purchase_cost:{minor:1000,currency:'CNY'},total_cash_profit:metric,actual_discount_bps:metric});
 assert.equal(els.get('summary-total-profit').textContent,'未知');assert.equal(els.get('summary-total-cost').textContent,'CNY 1000');assert.ok(els.get('summary-held-units').textContent.includes('待收 2'));
});
