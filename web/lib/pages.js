// Navigation and page metadata only. Trading rules stay in Rust application/domain.
export const groups = ['工作台','资产与研究','账户与自动化','拓展'];
export const pages = [
  {id:'dashboard',title:'仪表盘',group:0,kicker:'YOUR MARKET, IN FOCUS',description:'从行情、仓位到交易，让每一步都有依据。',icon:'grid'},
  {id:'workspace',title:'选品工作台',group:0,kicker:'01 / DISCOVER',description:'画好范围，理解每件商品的去留。',icon:'search'},
  {id:'planning',title:'预算计划',group:0,kicker:'02 / PLAN',description:'在预算、报价和持仓约束下，比较本轮方案。',icon:'plan'},
  {id:'inventory',title:'库存管理',group:1,kicker:'ASSETS / INVENTORY',description:'观察账户库存，保留资产身份和读取范围。',icon:'box'},
  {id:'purchases',title:'持有饰品',group:1,kicker:'ASSETS / HOLDINGS',description:'区分已持有、已挂牌与未决采购。',icon:'layers',kind:'holdings'},
  {id:'purchase-history',title:'操作记录',group:1,kicker:'JOURNAL / RECORDS',description:'回看扫描与预算，保留当时的依据。',icon:'journal'},
  {id:'analytics',title:'数据分析',group:1,kicker:'RESEARCH / OVERVIEW',description:'分开观察盘口、现金报价和价格历史。',icon:'chart'},
  {id:'steam-research',title:'Steam 盘口',group:1,kicker:'RESEARCH / STEAM',description:'保留实际币种，查看公开买卖深度。',icon:'chart',child:true},
  {id:'buff-research',title:'BUFF 报价',group:1,kicker:'RESEARCH / BUFF',description:'读取候选的首屏卖单，明确可见价格与数量。',icon:'chart',child:true},
  {id:'price-history',title:'价格历史',group:1,kicker:'RESEARCH / HISTORY',description:'按自然日研究变化，保留缺口和未知。',icon:'chart',child:true},
  {id:'sales',title:'上架记录',group:1,kicker:'ASSETS / LISTINGS',description:'跟踪资产上架、确认、撤单与成交。',icon:'tag',pending:['按账户同步真实在售资产','记录上架、确认与撤单结果','将平台成交与采购资产逐项对账'],related:'inventory'},
  {id:'accounts',title:'账号管理',group:2,kicker:'ACCOUNT / SESSIONS',description:'管理账户与登录态，在官方页面完成身份验证。',icon:'user'},
  {id:'steam-guard',title:'Steam 令牌',group:2,kicker:'ACCOUNT / GUARD',description:'管理 Steam Guard 与自动登录流程。',icon:'shield',pending:['Guard 自动登录与挑战状态','令牌安全存储与后台续期'],related:'accounts'},
  {id:'strategies',title:'策略中心',group:2,kicker:'AUTOMATION / STRATEGIES',description:'保存购入研究规则，用明确证据逐项验证。',icon:'sliders'},
  {id:'proxy',title:'代理池',group:2,kicker:'SYSTEM / NETWORK',description:'管理平台连接、代理路由与可用性。',icon:'globe',pending:['代理节点与路由配置','连通性检查及平台访问策略'],related:'accounts'},
  {id:'settings',title:'系统设置',group:2,kicker:'SYSTEM / PREFERENCES',description:'查看当前运行方式与配置边界。',icon:'sliders',kind:'settings'},
  {id:'debug',title:'运行日志',group:2,kicker:'SYSTEM / LOGS',description:'追踪运行事件和失败原因。',icon:'terminal',pending:['脱敏事件日志与任务状态','日志筛选及导出'],related:'purchase-history'},
  {id:'steam-deals',title:'Steam 折扣',group:3,kicker:'EXPLORE / DEALS',description:'关注游戏折扣、区域价格与历史低价。',icon:'tag',pending:['折扣与区域价格采集','历史低价、评分和折扣卡片'],related:'dashboard'},
  {id:'gift',title:'Steam 赠礼',group:3,kicker:'EXPLORE / GIFTS',description:'管理游戏版本、好友与赠礼流程。',icon:'gift',pending:['好友和游戏版本查询','购物车与赠礼流程','区域和钱包条件核验'],related:'dashboard'},
];
const aliases = {auto:'dashboard',history:'purchase-history',roadmap:'dashboard',holdings:'purchases',records:'purchase-history'};
export function resolvePage(hash) {
  const key=hash.replace(/^#/,'')||'dashboard';
  return pages.find(p=>p.id===(aliases[key]??key))??null;
}
