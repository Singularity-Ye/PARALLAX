import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import {readFileSync} from 'node:fs';
const code=readFileSync(new URL('../../web/lib/cockpit.js',import.meta.url),'utf8').replace(/^import .*;\r?\n/gm,'').replace(/export function/g,'function');
function setup(){
 const elements=new Map();
 function element(tag='div',text){return {tag,textContent:text??'',children:[],style:{},attributes:{},hidden:false,disabled:false,_value:'',
  get value(){return this._value||this.children[0]?.value||''},set value(v){this._value=v},
  append(...nodes){this.children.push(...nodes)},replaceChildren(...nodes){this.children=nodes;this._value=''},setAttribute(k,v){this.attributes[k]=v}};}
 const get=id=>{if(!elements.has(id))elements.set(id,element());return elements.get(id)};
 const context=vm.createContext({document:{getElementById:get,createDocumentFragment:()=>element()},node:element,console,Date,Number,Math});
 vm.runInContext(code,context);return {context,get};
}
const item={item:{key:'a',name:'A <unsafe>',cash_cents:387,steam_gross_cents:547,volume:200,observed_at:1000},passed:true,ratio_bps:8148,valuation:{net:{currency:'CNY',minor:475},fees_minor:72}};
function report(candidates=[item]){return {request:{source:'steamdt',wallet_policy:{kind:'legacy115'}},coverage:{accepted:1,examined:1},candidates}}
test('candidate rendering uses backend fields and text nodes, never legacy invented names',()=>{
 const {context,get}=setup();context.renderCockpitScan(report());
 const row=get('dashboard-candidates-tbody').children[0].children[0];
 assert.equal(row.children[0].textContent,'A <unsafe>');assert.equal(row.children[1].textContent,'CNY 5.47');assert.equal(row.children[5].textContent,'预筛通过');
 assert.equal(get('friction-net').textContent,'CNY 4.75');assert.equal(get('friction-fee').textContent,'CNY 0.72');assert.equal(get('friction-ratio').textContent,'81.48%');
});
test('missing valuation remains unknown; zero fee is a valid observation',()=>{
 const {context,get}=setup();context.renderCockpitScan(report([{...item,ratio_bps:null,valuation:null}]));
 assert.equal(get('friction-net').textContent,'未知');assert.equal(get('friction-fee').textContent,'未知');assert.equal(get('ratio-marker').hidden,true);
 context.renderCockpitScan(report([{...item,valuation:{net:{currency:'CNY',minor:547},fees_minor:0}}]));assert.equal(get('friction-fee').textContent,'CNY 0.00');
});
test('empty next scan clears old friction evidence',()=>{
 const {context,get}=setup();context.renderCockpitScan(report());context.renderCockpitScan(report([]));
 assert.equal(get('friction-cash').textContent,'未知');assert.equal(get('ratio-marker').hidden,true);assert.equal(get('friction-item').disabled,true);
});
test('water level reflects saved allocation and manual reserve, never account balance',()=>{
 const {context,get}=setup();context.renderPlan({id:'plan-1',created_at:1000,source:'demo',request:{budget_cents:10000,max_units_per_item:2,max_positions:3},cash_total_cents:6000,reserved_cash_cents:1000,remaining_budget_cents:3000,estimated_wallet_total_cents:7500});
 assert.equal(get('capital-planned').style.width,'60%');assert.equal(get('capital-reserved').style.width,'10%');assert.match(get('dashboard-plan-scope').textContent,/非余额/);assert.match(get('sentinel-position').textContent,/不是零持仓/);
});
test('inconsistent allocation hides proportions rather than fabricating a full bar',()=>{
 const {context,get}=setup();context.renderPlan({id:'plan-2',created_at:1000,source:'steamdt',request:{budget_cents:10000,max_units_per_item:2,max_positions:3},cash_total_cents:6000,reserved_cash_cents:0,remaining_budget_cents:0,estimated_wallet_total_cents:7500});
 assert.equal(get('capital-planned').style.width,'0%');assert.match(get('capital-track').attributes['aria-label'],/未显示水位/);
});
