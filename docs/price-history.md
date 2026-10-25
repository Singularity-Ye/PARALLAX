# Parallax 价格历史与自然日研究

状态：已接 Steam 只读 pricehistory 适配、账户/扫描候选或库存快照绑定、UTC 日聚合/P10及不可覆盖快照；新增IQR/CV/EMA/自然日趋势/R²基础描述，见 [stability.md](stability.md)。**当前102项常规本地测试通过、2默认忽略；Steam单账户登录与一个真实候选的UAH历史读取已通过；其余协议分支仍待验收。** 研究未自动改变扫描、预算或交易策略。

## 原项目思路与复用

继承 `steam/client.py:fetch_history`、`app/services/steam_client.py`、`analysis/stability.py` 和 audit 的 `candidate_report.py`。保留商品历史、日量加权、七日分位与压力滑点，修正记录索引冒充自然日、缺量补1、猜币种/默认FX和无历史默认-8%。IQR/CV/EMA/自然日趋势/R²与历史区间位置已接基础描述；完整快慢卖、动态阈值/布林带及策略条件仍待迁移。

先检查现有 reqwest、Accounts、risk、SQLite。社区端点交叉核对来源：[InternalSteamWebAPI 协议记录](https://github.com/Revadike/InternalSteamWebAPI/wiki/Get-Market-Price-History)、[Go Steam 客户端的历史响应定义](https://github.com/doctype/steam/blob/master/market.go)；这些是第三方实现/记录，不能代替 Valve 官方合同或平台样本。没有引入 Go/Node 运行时或第二套登录态。

日历和时区换算复用 [chrono 0.4.45 的解析/固定偏移接口](https://docs.rs/chrono/0.4.45/chrono/struct.DateTime.html)（MIT/Apache-2.0），关闭默认功能，仅启用 std，不依赖系统本地时区。serde_json 启用 arbitrary_precision，保留 JSON 数值文本后做整数十进制转换，避免经过 f64。新锁定包为 chrono、num-traits、autocfg；其余复用现有依赖。

## 模块与认证

- `domain/history`：请求、固定精度样本、范围、研究结果与快照；复用 `risk` 的日聚合与 `calendar_p10_v1`，新增内部可缩放聚合入口，不建立平行风控公式。
- `application/history`：确认已完成的真实 SteamDT 扫描与候选，获取/分析/保存、账户历史；新 HistoryProvider/HistoryRepository。
- `application/accounts_steam`：从库存代码抽出 SteamAccess，它持有**现有 Accounts.gate**。库存与价格历史复用在线身份、凭据读取、轮换/失效/429 处理，无新会话状态机。
- `SteamReadPolicy`：在原 auth_policy 表按 scope 保存库存/价格历史节流，旧 steam_inventory 键保留，兼容原库。429 也更新账户会话冷却。
- `adapters/steam_history`：平台 GET、有限解析；`adapters/steam_read` 共用状态/重定向/认证 Cookie 分类。SQLite 新增 price_history_snapshots，不改旧表数据。
- `transport/history` 与 `web/price-history.js`：只做输入、输出、错误反馈和历史选择，反重力仍可沿用统一 API 美化。

读取要求最近 300 秒内 Steam valid，subject 匹配账户 Steam ID，checked_at 非未来，账户/相应 scope 冷却结束；先拒绝演示、错候选和无账户，再取凭据。整次读/分析/保存持有原认证门，导入、停用、验证和浏览器恢复需等待。历史查询不读凭据，不刷新有效性；旧代次只是当时证据，不是执行权限。单进程门不保证跨进程事务。

## 操作与 API

`./dev.ps1 run`，打开 `http://127.0.0.1:38480/#price-history`。创建账户 → 获取并在线核验 Steam 登录态 → 等待冷却 → 完成真实 SteamDT 扫描 → 选候选和窗口 → 显式读取。不会自动登录、重试、后台轮询或请求演示商品。

| 方法 | 路径 | 行为 |
| --- | --- | --- |
| POST | /api/accounts/{account}/price-histories | 读取并保存 201 |
| GET | /api/accounts/{account}/price-histories | 此账户最近十份完整研究快照 |
| GET | /api/accounts/{account}/price-histories/{id} | 不变的历史详情，跨账户 404 |

请求示例：

```json
{"scan_id":"completed-real-scan-id","item_key":"Prisma 2 Case","window_days":90,"slippage_bps":300}
```

window_days 必填，8–365；slippage_bps 必填，0–10000。窗口是请求开始时刻之前已结束的 UTC 日期 `[today-window_days,today)`；开始/完成时间分别保存，跨午夜不会将未完整当天混入。响应保存候选请求、账户/凭据代次、identity_binding、series 和 analysis，platform_verified=false 标记当前协议尚未完成平台验收。前端类型明确区分 source 的 1e4 精度与 Money 最小单位。

body 最多 2 KiB，同源写检查，所有响应 no-store。输入 400/422，不存在 404，身份/代次/冲突 409，冷却 429+绝对 retry_at，其余平台/格式/网络失败 502。失败不保存新快照、不退回演示；旧结果保留并明确是历史。列表刷新失败不冒充保存失败。

## 来源、时间与金额

固定 `GET https://steamcommunity.com/market/pricehistory/`，appid=730、market_hash_name 为扫描候选原 key、l=english。响应不回传商品名，因此身份是扫描候选与请求参数绑定，不能声称平台签名确认商品身份。不开 TLS 例外、不跟随跳转、不自动重试；20 秒、8 MiB、最多 100000 个响应样本，窗口内最多 20000 个。请求前写两秒节流；429 冷却 300 秒至七天，重启/新账户/换 Cookie 不清除对应全局冷却。

403 是拒绝读取，不直接判会话失效；401/同主机登录跳转才置 invalid，认证 Cookie 变化置 unverified 并更新代次。网络/格式失败不改在线核验时刻。错误只回静态脱敏信息，不回显 Cookie、UA、平台原文或响应 HTML。

要求 success=true/1 与 prices 数组；显式 [] 是空历史，缺字段是失败。每条为日期、价格、选填成交量；英文日期必须携带显式 UTC 偏移，支持 +0/+8/+0800/+0530 等合法偏移，转换为 UTC。无时区、非法日期、未来点和重复 UTC 时刻均拒绝，不靠主机时区补齐。不按输入顺序假定时间先后；排序后保存。窗口外数据也校验格式，不通过过滤悄悄吞错；分别统计窗口前与当天排除数。

source `price_major_1e4` 是每个大单位 10000 个整数单位，**不是分**。只接正常十进制、最多四位小数的数值或字符串；零价、负数、指数、过多精度或溢出拒绝。volume 缺失/null 为未知，0 仍是明确零；正整数才参与加权，不补 1。日聚合后才向下量化为已支持币种的最小单位；低于该单位的日价标未知并计数，不能变成零价机会。

仅接明确 ISO 代码组合或已适配的无歧义标志：CNY/USD/GBP/EUR/HKD/TWD，当前这些金额按两位最小单位处理，仍待平台核验。单独 `$`/`¥`、未知标志或其他币种不猜；保留 source 样本，currency=null，不生成货币日线/P10。不使用请求币种、旧账户硬编码或默认汇率来补救。币种识别是小范围规则，未知情况需补样本再适配。

## 日线与风险含义

只聚合有记录的 UTC 日期；缺日单独计数，不插值、不当价格不变。VWAP 只使用已知正量，零/缺量样本计数保留。平台价格桶可能是中位价而非逐笔均价，所以结果是**日量加权样本价**，不是已证明的逐笔 VWAP。

七日收益仅配 `day` 与 `day-7` 都有有效价的日期；整数 bps 下取整，P10 沿用有版本的 floor-index 顺序统计，不引入插值。没有配对时 return_p10/conservative_gross 为 null，不使用 -8% 假历史。压力参考价是最后一个有有效价的历史日，明确保存 reference_utc_day；只应用 P10 的下行部分与请求滑点，不是实时出售价、净到账或未来损失上限。样本个数必须保留，少量配对不代表策略已校准。

PlanRequest/PlanPreview 不变，不自动刷新 Steam/BUFF 报价、延长时效、给出增购量或修改预筛。后续候选研究与策略引用需另定币种、来源、时效和样本门槛。

## 验证与下一步

命令：`cargo test --locked --offline`、`cargo fmt --check`、`cargo clippy --locked --offline --all-targets -- -D warnings`、`node --check web/price-history.js`。初次历史切片新增11项合成测试，当时79项通过；当前新增7项稳定性/兼容测试，完整102项通过、2项默认忽略，HTTP/不可覆盖/重开与所有原模块回归通过。真实平台未验收。

下一步：真实历史字段/币种/精度/空响应样本，以及动态阈值/布林带/版本化策略与显式指标引用；同时继续挂牌/未决来源与持仓合并、完整预检/订单。历史描述指标不等于实盘策略已完成。

## UAH单位增量

已适配明确UAH/₴/грн/грн.标记的乌克兰格里夫纳，冲突仍未知。先保留source1e4精度、按自然日聚合，再量化为百分之一UAH；日线/EMA/P10仍原币，不用今天汇率改历史形状。见 [uah.md](uah.md)。真实乌区登录历史仍待验收，本轮相关解析使用夹具。

## 2026-10-25 真实行情与乌区历史验收

复用现有API完成SteamDT单页真实扫描：10条候选，5条通过、5条拒绝。随后对首个真实候选读取90天Steam价格历史，保存成功；响应明确识别为UAH，响应1351条，窗口内693条样本。此为单个真实商品样本，未证明所有日期格式/币种/商品分支均通过，也不证明钱包费率或结算。

匿名盘口请求UAH时，平台实际返回eCurrency=8，现有适配器拒绝未支持币种，未保存误标UAH的报价。账户历史UAH与匿名盘口返回币种分别记录，未改变账户地区、未猜汇率、未下单。

## 库存商品历史入口（2026-10-25）

库存页打开保存的快照，对有市场名且reported_marketable=true的饰品点击“研究价格历史”，跳转历史页并带入账户、库存快照与商品。仅准备表单，点击读取才请求Steam；不会自动遍历库存、刷新库存或访问BUFF。旧库存仅证明当时观察到此名称，不证明当前持有、可交易或出售授权。

同一POST /api/accounts/{account}/price-histories增加互斥来源：原scan_id保持兼容，或只传inventory_snapshot_id。示例：

```json
{"inventory_snapshot_id":"saved-inventory-id","item_key":"exact-market-name","window_days":90,"slippage_bps":300}
```

来源必须且只能有一种；库存快照必须属于请求账户、商品精确匹配且可上市场。部分库存也可研究其中明确观察到的商品，不将局部范围变为完整仓位。无效来源在取凭据/访问平台前拒绝。沿用认证代次、五分钟在线身份、冷却、历史适配、分析和不可覆盖存储，无新依赖或表。

结果identity_binding为inventory_snapshot_and_request_parameters，保存inventory_snapshot_id；旧扫描结果仍为scan_candidate_and_request_parameters，旧JSON可读取。此为商品级市场历史，不包含该资产成本或个人成交记录。采购策略仍要求原扫描绑定，库存来源历史不混入已有采购计划；持仓出售研究可按原商品/账户规则显式引用。

验证：新增来源互斥/旧JSON兼容、库存账户与精确市场商品绑定两项测试；跨账户、缺失、不匹配和不可上市均在平台调用前拒绝。采购策略原绑定测试扩展为拒绝库存来源。完整186通过/2忽略；真实页面单个库存商品UAH历史保存成功（窗口内763条、90个有效日），未访问BUFF或创建订单。
