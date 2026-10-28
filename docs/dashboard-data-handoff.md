# Dashboard数据展示模块交接（Codex → Antigravity）

日期：2026-10-28。用户明确指定：由反重力统一集成，Codex只交接数据模块。基线为main / 23167220e5fc54dfcae0c49557e83f92a8b85d47。本说明不代表界面已集成或服务已部署。

## 本次Codex独立交付文件

- web/lib/cockpit.js：预算/扫描证据展示模块，复用parallax API及pricing-view的node；无新增依赖，无业务写请求，不发平台采集请求。
- tools/tests/cockpit.test.mjs：Node内置测试，5项通过。
- 本交接说明。

HTML/CSS、shell/app与HTTP路由当前均有未提交改动；其中包含双方改动。反重力接管统一核对，不要reset/整树覆盖，不将未审查内容直接全量暂存。Codex不继续编辑这些文件、不重启服务。

## 模块接口与接线

```js
import {initCockpit,renderCockpitScan} from './lib/cockpit.js';
// HTML节点完成后初始化一次。
initCockpit();
// 原parallax:scan-selected处理器中调用：
renderCockpitScan(report);
// 原renderPlan(plan)中派发已选预算，不创建新预算：
window.dispatchEvent(new CustomEvent('parallax:plan-selected',{detail:plan}));
```

Rust需登记静态资源 /lib/cockpit.js（include_str），可沿用当前static_response缓存处理。路由与shell导入、app事件的初步接线已经留在工作树，但未提交/部署；请按最终布局审查再接纳。

initCockpit默认从本地plans列表加载最近保存预算（按created_at倒序），仅为历史预览；用户新选计划优先，不被晚到初始化响应覆盖。读取proxyStatus只说明后台模式，不进行节点探测；不读取Cookie、凭据或Steam登录态。UI改成显式选择预算也可以调整这一初始化策略，但不得将计划当账户余额。

## 真实字段映射

### 资金/仓位

PlanPreview.request.budget_cents、cash_total_cents、reserved_cash_cents、remaining_budget_cents、estimated_wallet_total_cents，均为CNY分。水位只有在非负整数且planned+reserved+remaining===budget时绘制；展示比例仅用于排版，不重算分配规则。

- 钱包指标叫“计划估算净钱包”，不是账户钱包余额/资产。
- 填报预留不是平台资金冻结。
- request.position_snapshot为手工仓位；未引用不等于零持仓。
- request.liquidity_policy说明该计划参数，不能泛称全局保护已启用。
- 来源（demo/参考）、计划ID和created_at保留，历史预览不标实时。

### 报价摩擦与折算标尺

ScanReport.candidates[].item提供key/name/cash_cents/steam_gross_cents/volume/observed_at；判定为candidate.passed；折算率为candidate.ratio_bps。原Dashboard使用的verdict、metrics.discount_ratio、buff_price、steam_price不是当前Candidate契约，须移除旧映射。

费用与净钱包直接采用candidate.valuation.fees_minor / valuation.net，只有明确CNY时显示CNY。缺valuation显示未知；0费用是有效观测。折算标尺0–200%，超界只截断图形，文字仍保留原值。它不是实时盘口滑点、成交收益或净利润；volume统计窗口未确认，不能写24h。

模块用文本节点构建表格，不插入商品名称HTML。扫描为空时清除上一份摩擦证据，避免旧值伪装当前；表只展示前8项，不表示完整市场。

## 必需DOM ID（可由反重力按统一样式重排）

- 预算KPI：dashboard-budget-val、dashboard-wallet-val、dashboard-reserved-val。
- 预算证据：dashboard-plan-scope、capital-track、capital-planned、capital-reserved、capital-planned-value、capital-reserved-value、capital-remaining-value、capital-position。
- 摩擦：friction-item、ratio-track、ratio-marker、friction-cash、friction-net、friction-fee、friction-ratio、friction-scope。
- 右侧参数：sentinel-position、sentinel-risk、sentinel-route。
- 原表格：dashboard-empty-candidates、dashboard-candidates-table、dashboard-candidates-tbody、dashboard-pass-rate。

如果最终HTML换ID，集中改本模块的绑定，不能留下空节点导致shell整个初始化失败。SVG/正文/层级与视觉由反重力决定，不需要复制本次试写的CSS；若不使用Codex样式，请只移除明确属于本轮的新增证据布局段，保留此前他人改动。

## 当前共享工作树中的试写内容

- index.html：新增capital/friction节点，右侧改为Spec列表，去默认账户身份/¥100可用余额/预先在线等假状态，移除⚡，修正扫描表头语义。
- shell.js：调用cockpit模块，使用真实Candidate字段，健康失败与账户读取失败如实反馈。
- app.js：renderPlan派发plan-selected。
- components.css：追加以“Dashboard evidence layout”开头的样式段（主题token复用），并修正响应式ID选择器；其前方Plan B Telemetry Row样式与侧栏修复是他方并发内容。
- proxy.js：将✓/✗/警告Emoji替换为文字。
- src/transport/http.rs：新增cockpit资源路由，其余static_response缓存改动是他方既存内容。

以上不在本次Codex独立checkpoint内，全部交由反重力逐项集成；不以未提交工作树当作最终基线。

## 验证及待验收

运行用户现有Node路径执行：node --test tools/tests/cockpit.test.mjs；5项通过。覆盖真实后端字段与文本节点、缺失值/0费用、空扫描清除、预算水位、金额不一致不绘图。cockpit/shell/app/proxy脚本语法检查通过。

测试用VM与DOM夹具，无真实平台/账户写请求；不是浏览器/可访问性验收，也没有重新跑Rust全量测试。最终集成需验证：所需节点完整、桌面左右对齐、窄屏/缩放可滚动、计划切换/无计划/读取失败、扫描切换/空扫描/未知费用、来源时间，以及静态资源与二进制一致。共享服务本轮未重启。

另有此前审查待办（不被本模块自动修复）：Purchases的1.30/0.85伪估值、快捷建仓缺client_key、持仓汇总/空仓错误、HoldButton重复提交风险等。它们应独立收口，不因本次视觉交接而改标完成。
