# UI 复用与 Antigravity 接手

目标是**清爽、少重复、状态清楚**。当前复用到主题、样式组件、页面元数据、统一API、币种选项、定价/费用表单与相同证据展示；还不是完整组件化的全站设计系统。没有引入新的前端框架、依赖或第二套业务模型。

## 从哪里开始改

| 需求 | 唯一维护入口 | 谁复用 |
| --- | --- | --- |
| 纸感、颜色、字体、字号/间距基准 | web/theme.css 的tokens | 全站 |
| 导航、卡片、表单、按钮、排版/响应式 | web/components.css 的共享类 | 全站；新表单继续用pair/fieldset/label |
| 导航名称、分组、页面状态 | web/lib/pages.js | shell.js；切页不重建DOM |
| 货币选项/中文标签 | web/lib/currencies.js | 两个盘口选择器和策略费用选择器，含UAH |
| 壁垒/断层参数 | web/lib/pricing-fields.js 的pricingEditor | 盘口研究、压力净卖出规则 |
| 原币费用参数 | 同文件feeEditor | 盘口研究、压力净卖出规则 |
| 剔除/断层证据 | web/lib/pricing-view.js 的pricingTrace | 挂牌参考结果、压力估值结果 |
| 退出估值 / 原币金额 | 同文件exitValuationView / nativeMoney | 购入研究、出售研究、台账 |
| 三态逐项解释 | web/lib/evidence-view.js 的decisionList | 预检、出售研究 |
| 下拉选择 / 时间 / 幂等重试输入 | web/lib/forms.js | 台账与出售页，台账三类写入 |
| 请求与字段契约 | web/lib/api.js / api.d.ts | 页面脚本；Rust才计算金额/策略 |

纸纹仅在theme.css用本地内联SVG定义，导航图标路径集中在shell.js的iconPaths，不复制图片/图标到各页；当前没有额外图片素材库，未来新增图片再统一归入web/assets并登记用途。

`style.css` 只导入主题与组件层。页面脚本负责选择证据、保存/加载和反馈，不各自维护一份费用字段/币种数组。`AGENTS.md` 已明确要求先查这些入口，避免后续复制一份分别美化。

## 组件怎么用

```js
import {pricingEditor, feeEditor} from './lib/pricing-fields.js';
const pricing = pricingEditor(document.getElementById('my-pricing'));
const fees = feeEditor(document.getElementById('my-fees'), {chooseCurrency:true});
// 初始化一次。加载已保存版本时显式set；切页不重新mount。
pricing.set(saved.parameters.pricing);
fees.set(saved.parameters.fees);
const request = {pricing:pricing.read(), fees:fees.read()};
```

组件只在传入容器内维护控件，返回read/set等小接口，不抓页面全局ID、不发请求、不存平台数据、不算金额。标签包裹输入，不生成冲突的全局ID。数值defaultValue正确设置，form.reset恢复初始值；两个实例共享定义、各自保留状态。

`chooseCurrency=false` 的费用编辑器由页面 `setCurrency(actual_book_currency)` 绑定实际原币；策略编辑器让用户显式选币种，后端要求与实际盘口/历史一致。币种选项是显示元数据，支持范围与小数单位始终由Rust验证。

## 美化时保留的行为

- 默认CNY、UAH可选、请求/实际/显示币种分开；不把变更标签当换汇。
- 已保存版本与未保存输入分开；切页保留，不因mount覆盖表单。
- 错误留在当前操作；列表刷新失败不能覆盖已保存成功的事实。
- 来源、时效、原币、缺失/未知、preview_only和未执行状态保留。
- 同一组件改完检查 `#steam-research` 与 `#strategies`；浏览器不重写费用/风险/预算算法。

静态资源嵌入Rust，改后 `cargo build --locked` 并重启本地服务；用自己的开发服务器时保持API同源代理。工程运行 `./dev.ps1 run`。主题换肤主要改tokens/共享类，结构调整改共享组件，页面特有流程留在页面脚本。

## 当前证据与后续

本轮新增`lib/evidence-view.js`，预检和出售研究共用三态解释/明细；`lib/forms.js`共用保留选择的下拉列表、本地时间与幂等重试键。台账/出售页继续复用主题、卡片、币种选项和原币金额格式，退出估值复用`exitValuationView`，没有在JS加入业务公式。台账`ledger.js`、出售`sale-research.js`各管自身流程，不重建整站/引入框架。刷新/切页保留收货勾选、表单；保存成功与列表失败分开。上架记录现含研究，不代表真实挂牌接口可用。合同见 [ledger.md](ledger.md) 和 [sell-policies.md](sell-policies.md)。

预检卡片位于策略中心，`preflight.js`只编排选择/加载/保存/回看，复用现有纸感tokens、card/section-title/secondary类及`pricing-view`的node/nativeMoney；金额/时效/策略仍在Rust。`api.js`/`api.d.ts`集中合同。没有另建主题或重复费用表单。空选择、切页、列表刷新和回看已验证，输入/原结果保留。详见 [preflight.md](preflight.md)。

浏览器检查：策略墙量25、定价页20，相互不串值；切回仍保留25，最终恢复20且未另存测试版本。新建表单重置恢复默认；原定价页使用已有TWD盘口计算原币费后结果正常。UAH选择器和schema2保存/unknown研究正常。完整Rust135项通过/2默认忽略，JS语法通过。

后续列表/状态反馈/其他表单仍有重复，出现第二个真实相同用例时继续提取，不为“复用率”造未使用的万能组件、平行状态机或整站重写。后续独立构建/发布确有需要时，再评估框架。初版视觉保持；本次没有改成新的美术风格。

## 资金与本地采购准备（2026-10-25）

操作记录新增 checkout.js 卡片，复用 theme/components、forms 的下拉/时间/按账户与意图作用域的幂等重试键，以及 pricing-view 的金额和统一 API/types。恢复意图时清空上一意图事件，不显示串单记录。后端计算资金/预留和证据；页面只展示。没有新增导航/主题/框架，仍为19入口、14含部分能力、5待接。

隔离合成库浏览器通过1000分观察→334分预留/666分剩余→刷新恢复→取消v2/1000分剩余和事件回看，控制台无error/warn。真实资金/平台订单尚未接。完整169通过/2默认忽略，合同见 [checkout.md](checkout.md)。

## BUFF订单观察入口

操作记录新增buff-orders.js，复用现有主题/卡片/forms/统一API，不在JS计算付款状态。精确ID查保存页；找到但未知、范围未找到、读取失败分别解释。数字3保持付款未知，PAYING标明仅保存时；会话门失败保留原输入/结果，切页保留。19入口/14含部分能力/5待接不变。182通过/2默认忽略，协议实测仍待做；见 [buff-orders.md](buff-orders.md)。
