# 默认人民币与原币换算

2026-10-24。默认显示与预算基准为 CNY，原始金额始终保留币种；当前已适配 CNY/USD/EUR/GBP/HKD/TWD 的百分之一最小单位。自动换算不会更改 Steam 账户、地区、钱包或平台实际结算币种。其他币种（包括零/三位小数单位）需明确适配，不能只加一个代码就当成两位小数。

## 来源选择与继承

原版 app/services/workers.py 用 open.er-api.com 拉 CNY 基准并倒数为外币兑人民币；sell_pipeline.py 先人民币策略定价再换回账户币种。保留这一业务意图，替换 float、无日期的固定fallback和缺失币种默认为人民币。

本轮短选型比较：

| 候选 | 许可/维护与接入 | 选择 |
| --- | --- | --- |
| ExchangeRate-API Open Access | 原项目使用；无需Key、日更新、可缓存但须署名，不允许作为自动汇率数据服务再分发；多币种支持 | 不作为当前共享API的默认数据源，避免再分发边界。见[官方文档](https://www.exchangerate-api.com/docs/free)、[条款](https://www.exchangerate-api.com/terms) |
| Frankfurter v2 blended | 官方当前v2、无需Key、支持CNY/TWD；默认混合日参考率允许使用及缓存，可自托管；不用于高频实时成交价 | 采用官方混合参考率，复用现有reqwest/chrono/serde/SQLite，不增加依赖。[官方文档与使用说明](https://frankfurter.dev/) |

实际请求固定 `https://api.frankfurter.dev/v2/rates`，base=CNY、quotes=USD,EUR,GBP,HKD,TWD、date=请求开始时UTC当天。核对时 latest 曾返回晚于本机日期的行，因此显式限制as-of日期并验证不能在未来，不用未来数据校正本机时钟。真实显式日期样本已成功；本轮未请求Steam/BUFF平台。

## 模型与复用

- domain/fx：FxSnapshot(id/source/fetched_at/rates)，每个币种保存原始日期与 units_per_cny_1e8；CNY锚点固定1。当前各币种单位相同，from→to 使用 `原币最小单位 × rate[to] / rate[from]`，不倒数成浮点。
- application/fx：共享ExchangeRates/FxResolver；OrderBookResearch和Application共用一个实例。持锁合并并发请求，六小时内复用数据库最新有效快照；失效时自动拉取并不可覆盖保存，失败暂停五分钟，不采用过期/fallback值。失败冷却为进程内状态，缓存跨重启保留。
- adapters/fx：固定官方HTTPS、正常TLS、不跟随跳转、20秒/64KiB、无凭据/代理轮换或自动重试；仅接JSON数值十进制最多8位，整数运算保留精度。
- SQLite兼容添加exchange_rate_snapshots，旧表/快照不改。日期逐币校验，不能未来或超过7个自然日；fetched_at不能未来或超过六小时。来源、全部六币种、CNY锚点、正数/上限均验证。

## 接到哪里

### 盘口显示

POST /api/steam/order-books/{id}/valuation，body `{}` 默认人民币，或 `{"display_currency":"USD"}`。200/no-store返回原snapshot_id、observed_at、native_currency、display_currency、折合best_ask/best_bid与完整fx。选择同币种不需汇率；异币缺少有效缓存会自动读取公开汇率。换算不重抓盘口、不更新其观察时间。界面异步响应与当前选择绑定，过时响应不覆盖新币种。

### 定价研究

沿用 listing-preview，新增可选 display_currency（默认CNY），策略policy.currency仍为CNY。外币用精确有理数比较人民币分段及绝对断层阈值，相对阈值仍对原币价差比较；不先量化/合并成假CNY盘口。算法 wall_gap_fx_v1；同币继续wall_gap_cny_v1。

压价/偏移/下限先按人民币基准作用于有理数价格，再向上取整到原币最小单位以保持下限，展示/预算估值向下取整。离散单位可能让实际压价小于所设人民币步长，故是研究参考，未保证符合平台实际价格步长。墙/剔除/gap证据仍原币，absolute_threshold_minor始终CNY分。返回native_gross_listing_estimate、CNY gross_listing_estimate、display_gross_listing_estimate和display_net_estimate及完整fx。

费用模型必须是盘口原币，先用既有FeeSchedule算原币净额，再折合显示；不把CNY最低费套到TWD。选择新快照会关闭可选费用模型，启用时明确标注原币。参数需要用户/未来账户模块提供，不声称默认模型就是平台实际费用。

### 预算预览

显式引用的外币盘口会自动取得有效汇率，领域规则将最低卖价/最高买价折CNY，仍核对商品、盘口时效、跨盘与BUFF独立时间。PlanPreview新增可选fx（完整快照），steam_valuations保留native_ask与CNY估值。旧JSON缺字段仍可读；历史查询不重新拉汇率、改原币或重算计划。所选CNY费用/1.15仍只是原预算模型估算，不能当外币账户真实结算费用。挂牌参考没有自动代替最低卖价预算。

API错误：业务币种/金额/费用不匹配400，缺失快照404，JSON类型400/422，异源403；汇率网络/格式失败502，刷新失败冷却中409。输入最多2KiB，换算结果上限10^12最小单位。缺汇率和估值失败留在当前操作，保留输入与原币数据。当前真实交易继续501。

## 验证与范围

本轮新增7项测试：精确方向/交叉换算、未来/过期/缺失/小额、适配器日期与JSON精度、并发合并/不可覆盖/数据库重开、失败冷却、原币断层/等号/floor、外币显示/费用/HTTP/预算贯通和持久化。94项完整回归及新增贯通定向1项通过（当前合计95项），2项默认忽略；Clippy/fmt/JS与debug构建通过。旧CNY路径继续回归通过。

已实测公开FX源与现有TWD快照的人民币/美元显示、外币挂牌参考；预算贯通用合成扫描与快照，未在用户DB添加模拟候选或计划。真实Steam/BUFF登录、实际汇兑/费用和成交执行仍未验收。历史风险、账本与其他页面后续复用本模块，不宣称全项目所有金额视图已经可切换。

运行 `./dev.ps1 run`，访问 `http://127.0.0.1:38480/#steam-research`。没有Key配置或新Node构建流程。
