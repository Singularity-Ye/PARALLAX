# Parallax 原项目设计审查记录

日期：2026-10-23。范围为本地 reference 与 audit 两份源码；基线及逐文件 SHA 见 `../source-inventory.md`。当前交付的是业务设计提取、关键路径审查与离线行为证据，不是所有函数、平台协议和原测试的最终验收。

## 审查方法与边界

- 围绕 24 组能力提取业务目的、输入输出、关键约束、故障处理和 Rust 边界。界面、侧功能与基础设施保留独立模块，不机械移植 Python 文件树。
- 关键源码直接阅读；原测试只用静态目录/符号索引定位既有合同，未运行原项目的完整测试套件。
- `tools/legacy_contract_probes.py` 仅抽取指定函数 AST，使用合成数据及假客户端，禁止旧应用模块启动。日期解析/币种转换在计算探针中被替换为纯夹具，不能据此声称真实解析/汇率已验证。
- `probe-results.json` 中 8 项观察可离线重复；L01 包含两版对照。网络协议、真实订单状态数字、浏览器状态传播等仍待平台夹具/实机核验。
- 此轮未修改 Rust 产品行为，未读取旧账号数据、运行旧后台任务、创建任何订单。

## 设计结论

保留“现金采购 → 资产收货 → 冷却 → 定价出售 → 钱包价值与结果分析”主链，扫描和计划作为前置用例。使用模块化单体，领域纯规则、应用流程、平台适配、存储与 HTTP/UI 各守边界。成熟库继续复用，语言替换不等于收益改善或网络请求提速。

必须保留：账户身份约束、报价新鲜度、费用/币种、仓位限额、购买前复核、写请求未知阻断、唯一资产归属、冷却时间来源、挂牌确认及结果账本。

应改写：全局 current_account/State、异构 JSON/SQLite 的整包覆盖、字典隐藏字段串联、用户确认冒充平台付款事实、散落的浮点金额/估算、跨账户无归属记录、无区分的重试和全量手机确认。

## 发现与继承决策

| ID / 证据等级 | 观察与源码入口 | 重制决策 |
| --- | --- | --- |
| L01 / 离线复现 | R `pipeline_steps._do_wait_payment_and_append` 在用户确认后可直接记账；A `_do_payment_notify_and_wait` 单笔路径调用 is_order_paid。不能再说 R 已完整实现付款核验 | 组合 R 持久化 intent/防重与 A 平台核验；UserReportedPaid 和 PlatformVerifiedPaid 分开 |
| L02 / 离线复现 | A `_do_payment_notify_and_wait` 邮件分支先用 threading.Thread，后在同一函数 import threading，导致 UnboundLocalError | 通知只是适配器；付款流程不嵌套邮件线程或共享无订单归属的回调 |
| L03 / 离线复现 | A `build_daily_curve` 零成交量记录按 1 加权进分子，分母有成交时只累加真实成交量；100×0 与 200×10 得到 210 | 按有效量一致加权，零量与缺量分开；日聚合值应受输入价格范围约束 |
| L04 / 离线复现 | A `candidate_snapshot` 的 return_p10=0 被 `or -1` 当作缺失 | Option 显式表达缺失；0 是有效观测 |
| L05 / 离线复现 | A `candidate_snapshot` 空卖单也返回 available_qty=1 | 库存 0/未知/有量分开，展示和计划不得伪造可购买量 |
| L06 / 离线复现 | A `seven_day_risk` 的 i 与 i-7 是七条数据间隔，夹具相隔 14 个自然日 | 风险窗口明确为日历窗口；缺日策略和样本数纳入算法版本 |
| L07 / 离线复现 | R `analyze_by_time` 用价格记录条数 count 与 days×min_daily_trades 比较，未用 total_volume | 分开 sample_count、volume、coverage_days；阈值名称和单位匹配 |
| L08 / 静态确认 | R `pipeline_steps._affordable_quantity`/`pipeline._process_deals_for_target` 的 target_balance 累计现金成本，且不可再买时 acc=target | cash_budget、cash_spent、target_wallet_net 分开；停止原因不能改写真实花费 |
| L09 / 静态确认 | R `accounts.add_account` 保存 password，`routes/accounts.api_list_accounts` 原样返回账户字典；`api_export_full` 含完整 credentials | 凭据与公开模型分离，默认导出不带秘密 |
| L10 / 静态确认 | R `steam_auth._verify_steam_cookies_valid` 部分异常返回 True；`steam/session.create_market_session` 默认 verify=False | unknown 与 invalid 区分；保留证书校验，Cookie 存在不等于认证有效 |
| L11 / 静态确认 | R `auto_confirm_once` 把所有确认项交给 accept_all | 只确认本次账户/订单/资产关联的挂牌，未知项人工复核 |
| L12 / 静态确认，风险待故障注入 | R `sync_sold` 读整表后 `replace_transactions`；数据库没有持仓账户字段 | 按稳定主键与版本更新、绑定 account_id；验证并发收货/同步时不会丢新记录 |
| L13 / 静态确认 | R `gift_engine._do_checkout` 和 `_grpc_request` 在异常时可能换代理重发写请求，未见对应持久化 intent | 侧功能复用请求结果未知与对账原则；另建礼物业务模型 |
| L14 / 静态确认 | R `analyze_by_time` 的 price_percentile 是 min-max 位置；部分 band 参数仅作开关使用 | 明确 range_position 与经验分位不同；策略参数必须实质参与计算 |
| L15 / 静态确认 | A 预检 token 绑定选择与 TTL，放在进程字典；未绑定不可变账户/凭据/策略版本 | 执行授权快照绑定账户、凭据代次、规则版本、数量、费用和报价时间 |
| L16 / 静态确认 | R `gift_engine` 购物车 payload 含硬编码区域片段，运行模式 server 默认监听所有地址 | 提取协议适配与显式地区输入；重制默认本机运行，远程部署另定认证边界 |
| L17 / 静态确认 | R `BuffClient.try_batch_buy` 返回安全降级；`BuffBuyer.batch_buy_create/batch_buy_finalize` 明确禁用旧协议 | 保留批次业务意图与完整性原则，不把保留的批量代码当作当前可用功能；新协议单独验证 |

## 各模块审查深度

- M01–03：账户/手动与浏览器入口、自动登录 pending、身份/币种、凭据代次与验证阻断主链已审阅；steampy 的底层协议仍需专门适配验证。
- M04–12：来源范围、行情/历史接口、费用/定价、策略契约、限购、候选/组合、预检关键路径已提取；并非所有参数组合都已跑过。
- M13–19：intent、防重、付款两版差异、收货匹配、挂牌所有权、确认/撤单、整表同步与账本主链已审阅；完整 batch/部分成交恢复、并发和平台枚举仍待验收。
- M20–21：产品目的、服务入口、断点采集、购物车/结账与写请求恢复风险已审阅；每个 protobuf 字段和绘图 helper 尚未逐项验证。
- M22–24：配置/导入导出、任务/后台 worker、运行模式与 UI 职责已提取；代理算法、邮件协议、前端各异常交互、容器和全旧测试未完成动态验证。

知识库的《原项目设计审查总览》及四篇分册维护业务解释与 Rust 继承合同；本文件保留工程证据。后续审查状态必须随新证据更新，不能把“思路已提取”改写为“全部代码已验收”。
