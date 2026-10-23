# Parallax 原项目能力与迁移地图

核对日期：2026-10-23。目标是覆盖原有能力，再逐模块迁移。本文描述设计与差距，不代表真实交易已接通。

## 证据范围

- `AetherSwap_reference`：HEAD `02552e5d9c7b2fe237ea908017819c9f0a09cb69`，含本地改动；180 文件、120 Python 文件。
- `AetherSwap_audit`：HEAD `4dcd80de7331b7fb1ffaddc253b7dc4d4b64ae64`，含本地改动；157 文件、105 Python 文件。
- 完整路径、Python 符号/行号、import、HTTP 路由、内置策略模块/参数字段、两版文件差异见 [source-inventory.md](source-inventory.md)。225 个 Python 文件均通过 AST 静态解析。两份来源有重复，不能把 337 文件当作 337 个独立模块。
- 配置和用户数据只登记路径，不读取账号内容、不执行旧项目。此轮基于本地快照，未确认远程最新变化。
- 已完成：全文件归类、符号/路由索引、关键交易链路阅读、能力差距设计。尚未完成：逐函数行为验收、所有旧测试的 Rust 等价迁移、平台实机验证。

## 能力划分与迁移状态

R 表示 reference；A 表示 audit。下表 Rust 路径为**目标设计**，只有注明已实现的部分已存在。

| 编号 | 能力与来源证据 | Rust 归属与复用边界 | 当前差距 / 验收依据 |
| --- | --- | --- | --- |
| M01 | 多账户、当前账户、地区币种；R `app/accounts.py`、`services/account_region.py`、`routes/accounts.py` | domain/accounts；application/accounts；账户仓储 | 部分实现：账户创建与显式账户隔离已接入；地区币种同步和账户管理待迁移 |
| M02 | Steam 浏览器获取登录态、手动 Cookie、账号密码/Guard 自动登录、轮询 token、失效恢复；R `routes/auth.py`、`services/steam_auth.py`、`steam/session.py` | domain/auth；application/auth；adapters/steam_auth、credential_store、browser_auth | 部分实现：手动 Cookie 与独立浏览器登录/人工恢复已接入；Guard 自动登录和后台续期待迁移，真实平台登录未验收，须保留 pending/rejected/network/expired 的区别；旧测试 `test_auth_relogin_state`、`test_steam_auth_errors` |
| M03 | BUFF 浏览器/手动登录、User-Agent、凭据代次、验证挑战；R `services/buff_auth.py`、`routes/auth.py`、`config_loader.py` | application/auth；adapters/buff_auth；复用账户与凭据端口 | 部分实现：手动导入、独立浏览器、只读身份核验和冷却已接入；浏览器核验并保存后才解除挑战；未决支付保护待迁移 |
| M04 | SteamDT 榜单、参数、分页解析；两版 `steamdt/*`、`services/iflow_client.py` | 现有 MarketSource、MarketItem、scanner、market adapter | 部分实现：分页、预算、去重、范围解释；非全市场目录，未实测实时来源 |
| M05 | BUFF 实价/卖单、Steam 买卖盘、nameid/SSR/分组商品解析、缓存；R `buff/buyer.py`、`services/buff_client.py`、`steam/market_orders.py`；A 增加买盘深度与价差 | domain/application/adapters 的 orderbook 与 buff_quotes；两类 provider/repository 端口 | Steam 匿名 TWD 样本通过；BUFF 账户绑定首屏适配、冷却/挑战与快照已实现但线上字段/CNY 未验收。SSR/分组页、histogram、FX 待迁移 |
| M06 | 手续费、币种、金额单位、展示价与到账价；R `utils/money.py`、`services/workers.py` 汇率任务 | domain/money；后续 adapters/exchange_rates | 参数化整数费用/最低费/步长逆推已接扫描和计划；保留默认 1.15 估算；真实费用与 FX 待验证 |
| M07 | 壁垒/断层定价、少量低档忽略、偏移；R `steam/market_orders.py:compute_smart_list_price` | domain/pricing；复用统一订单簿 | 未迁移；空盘口、单档、累计量、断层和价格下限需固定样例 |
| M08 | IQR/VWAP、CV、回归趋势/R²、EMA、历史分位/数据窗口；R `analysis/stability.py`、`services/analysis_client.py`、`utils/trend.py` | domain/risk；后续 application/analysis、HistoryProvider | 日 VWAP/七自然日 P10 纯函数已实现；缺数据返回未知；真实历史、CV/EMA/趋势待迁移 |
| M09 | 买卖策略注册、步骤顺序、依赖、参数、导入导出、声明式自定义条件、模拟；R `app/strategy_engine.py`、`routes/strategies.py` | domain/strategy；application/strategy；版本化策略仓储 | 未迁移；现有基础预筛不等于策略引擎；模拟结果不能充当实盘效果 |
| M10 | 采购硬上限、日量比例、低价惩罚、已有同名持仓扣减；R `purchase_limit.py`、`pipeline_steps.py` | domain/positions；复用订单/持仓快照 | 已接手动账户仓位、未决量、预留、可买量和日量约束；真实账户同步与执行风控待迁移 |
| M11 | 候选研究、慢卖/快卖/风险估值、七日 P10、压力场景、报价过期、目标游戏价格与余额组合；A `candidate_report.py`、`routes/candidates.py` | domain/valuation、portfolio；application/research；复用 scanner、报价/历史端口 | 预算显式引用 Steam/BUFF 快照，分别校验时间；BUFF 账户/代次绑定且只按最低档可见数限量。平台效果、风险组合与完整快/慢卖待验收/迁移 |
| M12 | 下单前复核、绑定具体选择的单次短时 token；A `routes/candidates.py`；R 购买前二次复核 | application/preflight；持久化不可变报价与选择摘要 | 未迁移；复核余额、库存、费用、价差、风险、账户和凭据代次；不能直接执行历史预览 |
| M13 | 单笔/批量锁单、付款提示、核销、写入未知阻断（R）；单笔确认后的平台付款/取消核验（A）；两版 `pipeline_steps.py`、`buff/buyer.py`、R `services/buff_checkout_guard.py` | domain/orders；application/checkout；adapters/buff_trade、order_journal | 未迁移，`POST /api/trades` 仍 501；R 单笔不能视为已完整核验付款；组合 A 核验与 R 防重，未知结果先对账 |
| M14 | 工作时间窗、目标余额、停止、互斥、维护锁、失败商品 TTL；R `pipeline.py`、`pipeline_context.py`、`state.py` | application/orchestrator、scheduler；复用纯规则与持久化订单 | 未迁移；区分停止新动作、等待已发请求和补偿/对账 |
| M15 | BUFF 发货贸易单、接受 Steam offer、购买记录到 asset 的唯一匹配；R `receive_flow.py` | application/receipt；adapters/steam_trade、buff_trade | 未迁移；模糊/多重匹配留待人工复核，不按名称随意绑定 |
| M16 | 库存分页、可交易时间、冷却、解锁元数据；R `steam/inventory.py`、`inventory_cs2.py`；A `trade_ledger.py` | domain/inventory；application/inventory_sync | 未迁移；estimated_unlock 与 actual_unlock 分开，解锁后保留真实时间 |
| M17 | 立即/趋势/收益保护/暂停出售、同名挂牌限额、上架确认、手机确认、撤单；R `sell_pipeline.py`、`steam/market.py`、`steam_confirm.py`、`steam_listings.py`、`steam_delist.py` | domain/sell_policy；application/listing；adapters/steam_listing、confirmation | 未迁移；绑定账户/资产、真实响应成功才记挂牌；避免全选确认不相关操作 |
| M18 | 成交同步、错误记录修复、交易 CRUD、持仓估值/报表；R `sync_sold.py`、`repair_error_records.py`、`database.py`、`notify.py`、`routes/transactions.py` | application/reconciliation；adapters/sqlite；审计事件 | 未迁移；保留证据、主键与重复检测，修复需 dry-run，不直接迁旧数据库 |
| M19 | 同批资产生命周期、真实折算率、滑点、持有时间、14 日解锁敞口；A `trade_ledger.py`、`web/js/tradeLedger.js` | domain/performance；application/ledger | 未迁移；余额价值和可提现现金分开，原阈值未经成交样本校准 |
| M20 | Steam 游戏折扣采集、区域价格、断点恢复、历史低价、评分、折扣卡片；R `services/steam_deals.py`、`deal_cards.py`、`routes/steam_deals.py` | 独立 application/deals、adapters/steam_store、card_renderer | 未迁移；与饰品自动买卖共用账户/汇率/任务，不耦合订单流程 |
| M21 | 好友、游戏版本、钱包、购物车 protobuf/gRPC、礼物结账；R `gift_engine.py`、`routes/gift.py`、`web/js/gift.js` | 独立 application/gifting；adapters/steam_cart | 未迁移；保留原能力作为后续边界，不纳入饰品订单状态机 |
| M22 | 配置校验/导入导出、SQLite/旧 JSON 迁移、通知/邮件、日志导出、代理/节流/重试、网络检查、任务队列、后台工作者、运行模式/桌面壳/容器 | application/jobs、notifications；adapters/config、network、runtime；transport | 仅本地 SQLite 扫描与计划/HTTP/嵌入页面已实现；旧桌面壳与 Docker 需单独迁移决策 |
| M23 | 账户/Guard/策略/交易/代理/配置/礼物/折扣/控制台 UI、主题、audit 候选与账本页；两版 `web/*` | 前端调用统一 HTTP API，业务计算留 Rust | 当前有扫描、预算/历史、手动会话、自填费用与仓位 UI；其余待迁移；统一客户端可供反重力复用 |
| M24 | 单元/集成测试、启动脚本、依赖、GPL/NOTICE、示例素材 | tests + 工程配置；素材不作业务代码迁移 | 当前 58 项常规测试通过；BUFF 用合成夹具，未在线验收；Steam/浏览器/凭据沿用前阶段证据；旧测试未逐项移植 |

## 必须纠正的几处认知

后续关键路径审查与 8 项离线观察见 [设计审查证据](audit/design-review.md)。重制提取业务目的和约束，不要求机械保留原文件结构；设计提取不等于全部实现细节验收完成。

1. 原工程不只有筛选、预算、买卖三个模块。身份、支付核验、收货映射、冷却与成交账本是主流程的必要部分。
2. 两版能力互补。reference 有更严格的凭据/checkout 日志、批量完整性、所有权和响应处理；audit 有候选人工复核、买盘价差、P10 风险和结果账本，不能用旧分支覆盖新安全修复。
3. 当前基础预筛对比例用 `<=`，原购买验证是 `value_ratio < max_discount`。重制应明确扫描展示边界与执行严格阈值，补等号边界用例，不能把扫描通过当成可下单。
4. `value_ratio = cash / (gross / 1.15)` 是现金换 Steam 钱包价值的折算率，越低通常越划算；不是商品折扣，也不是扣除全部风险后的投资收益。
5. 原 `seven_day_risk` 实现按有数据的第 i 与 i-7 行计算，缺日时不一定是七个自然日。移植前需选定对齐日历/缺口策略，并记录与旧算法的有意差异。
6. 原 Steam Cookie 校验会在部分网络异常时返回 True，且会话默认关闭 TLS 验证。这些是已发现的问题，重制要改为可区分的 unknown 状态和正常证书校验。

## 核心合同与依赖顺序

```text
账户与会话 → 实时报价/历史 → 费用与策略 → 持仓限额/候选计划
                                            ↓
账本/对账 ← 成交 ← 上架/确认 ← 库存冷却 ← 收货 ← 预检/锁单/付款核验
```

已实现端口：MarketSource、Repository、CredentialStore、AccountRepository、SessionVerifier、BrowserAuth、OrderBookProvider/OrderBookRepository、BuffQuoteProvider/BuffQuoteRepository。后续按需要补 HistoryProvider、TradeGateway、OrderRepository、InventoryProvider、NotificationSink；只在实现对应切片时添加，不提前堆空接口。

- **会话合同**：账户 ID、平台、凭据代次、状态和验证时间；密钥仅存凭据后端，API 只回脱敏状态。网络超时不等于凭据失效。浏览器和自动登录都交付相同会话对象。
- **报价合同**：原币最小单位、汇率时间、商品标识、买卖深度、可买数量、来源/抓取时间、估算/真实净值。CNY 展示与原币结算分开。
- **仓位合同**：总现金预算、本次数量上限、日成交量限制、同名已有持仓、已挂卖、未决采购与预留资金。现有预算 preview 不能作为该合同的替代。
- **执行合同**：持久化 intent 后再发请求；账户/选择/数量/报价版本一致；待付款、已核验付款、请求未知、已取消分别建模。恢复时先读平台状态再决定后续动作。
- **账本合同**：一条采购 lot 关联现金成本、平台订单、Steam offer/asset、真实解锁、挂牌/成交及报价快照；审计事件不可覆盖。
- **前端合同**：业务规则和状态来自 Rust；浏览器登录、人工付款/验证、未知结果的可继续操作有明确入口。反重力可独立换布局。

## 策略细节继承清单

- 原安全限购独立函数：日量乘流动性比例取整，低价时再乘惩罚，和硬上限取最小值；主流水线还有按模块启用与同名持仓扣减逻辑。不能只移植 `purchase_limit.py` 就宣布限购完成。
- 原盘口定价：剔除低量最低档 → 累计到墙量 → 小阻挡量内检查价格断层 → 常规压价/跳档 → 偏移、价格下限。保留解释文本与样例。
- 原七日风险：取收益分布 P10，下行部分乘参考价，再扣 3% 滑点；无样本时用 -8% 标记兜底。原阈值如 0.82、-12% 只作兼容基线，不作为已验证盈利参数。
- 原 audit 预检 token：90 秒、一次性、绑定选择；需要迁移行为，存储不能继续只依赖进程内字典而忽略重启和并发。
- 原费用代码含最低费、平台费/发行商费、价格步长与逆推；需要钱包参数与官方响应样例确认各币种语义，禁止把硬编码钱包样例当真实账户参数。

## 实施顺序与每步验收

| 顺序 | 切片 | 完成标准 |
| --- | --- | --- |
| 1 | 账户/会话与本地凭据接口，先手动导入+验证，再浏览器/自动登录 | 脱敏、账户隔离、登录 pending、网络失败/过期区分；不读取旧凭据；实现浏览器前对 Rust/CDP 等成熟方案做短选型 |
| 2 | 费用、折算率、盘口、稳定性、限购纯规则 | 从两版旧测试抽固定样例；正常/低价/缺币种/空历史/边界数值，与旧行为差异有记录 |
| 3 | 行情/库存/历史只读适配与统一研究 API | 夹具验证与平台验证分开，真实币种和 asset 匹配正确；再将规则接到现有扫描与计划 |
| 4 | 持久化订单、预检、付款核验、取消/未知结果恢复 | 故障注入覆盖请求前/后崩溃、重复确认、批量不完整；交易可追溯、不会重复买入 |
| 5 | 收货、冷却、挂牌/确认/撤单、成交对账与账本 | 单一 lot 全生命周期串通，账户归属正确，历史状态可恢复，真实结果另行验收 |
| 6 | 策略编辑、任务/通知/代理/配置，以及折扣游戏/礼物/卡片 | 按 M09、M14、M20–24 逐项补齐，不把侧功能静默删掉 |

## 本轮验证与剩余工作

运行 `python tools/source_inventory.py` 生成完整索引，检查两版所有登记路径存在、225 个 Python 文件可解析。该脚本不验证外部接口、策略收益或原测试通过情况。没有启动登录、付款、下单、礼物结账等操作。

当前最新为 BUFF 现金端阶段 58 项常规测试通过，19/30/38/48 是前阶段历史数字；Steam 匿名样本、独立空白浏览器与原生凭据沿用前阶段证据。BUFF 已有只读适配、共享认证门、快照与预算引用，本轮未使用真实会话或发出带凭据请求；真实登录、线上报价字段/CNY、库存/历史、自动续期和交易仍待验收/实现。合同见 [buff-quotes.md](buff-quotes.md)、[orderbook.md](orderbook.md) 与 [accounts.md](accounts.md)。
