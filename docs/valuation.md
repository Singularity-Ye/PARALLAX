# Parallax 金额、估值与仓位预览 v2

后续已增加可选 Steam 盘口引用：PlanRequest.steam_snapshot_ids / PlanPreview.steam_valuations，详见 [公开盘口合同](orderbook.md)。只替换明确引用的有效 CNY Steam 慢卖估值，现金端仍保留原榜单价格与时间；以下费用与仓位规则继续适用。

## 已实现与边界

- `domain/money.rs`：带币种的整数金额、同币种合计、可配置费用逆推及向上取整的折算率。扫描与计划使用同一模型。
- `domain/positions.rs`：账户手工快照、已有持仓/未决买入扣减、明确的可买数量与可选日量限制。已接计划 API 和页面。
- `domain/risk.rs`：UTC 日聚合、量价一致加权、按相隔七个自然日配对的 P10 和滑点保守价；目前是可复用纯函数，**尚未接真实历史来源，也没有被默认为扫描通过条件**。
- `domain/planning.rs`：`ratio_greedy_v2`，区分现金、预留与新增钱包目标；不是全局最优组合，也不创建订单/预留资金。

没有迁移全套 CV/EMA/趋势策略、真实账户库存同步、精确平台费用验证或交易执行。真实交易继续返回 501。

## 费用合同

ScanRequest 新增可选 wallet_policy，省略时沿用 legacy115。两种模型都只是估算，参数必须与本轮扫描/报价快照一起保存。

```json
{"kind":"legacy115"}
```

```json
{"kind":"parameterized_fees","schedule":{"currency":"CNY","platform_bps":500,"publisher_bps":1000,"platform_min_minor":7,"publisher_min_minor":7,"increment_minor":1}}
```

第二段是**演示参数，不代表当前平台真实费用**。当前扫描/计划只接 CNY 分；其他币种参数明确拒绝。Money 类型允许三位大写代码，同币种才可相加，本轮未实现 FX 转换。

费用模型：`fee_component = rate == 0 ? 0 : max(minimum, floor(net * bps / 10000))`。在指定净值步长上二分搜索最大的正 seller_net，使 `net + platform_fee + publisher_fee <= gross`。不足以得到正净值时返回无值，不强制造出最低成交价。输出 net、fees_minor、gross_used_minor 和 unallocated_gross_minor。

legacy115 使用 u128 中间运算计算 `floor(gross * 100 / 115)`。折算率为 `ceil(cash * 10000 / net)`，避免截断后错误通过上限；零净值/比例不可表示时不通过。预筛阈值仍含等号，真实执行的严格阈值需后续独立设计。

新增 Candidate.valuation；旧扫描没有该字段仍可读取。新计划按保存的原始报价与扫描费用参数运行 v2 预筛，**不会改写旧扫描/旧计划**；不是重新拉取实时价格。

## 仓位快照合同

PlanRequest 新增可选 position_snapshot、liquidity_policy、max_position_age_secs（默认 300 秒）。snapshot 需要本地已存在的账户；来源固定标为 `manual_unverified`，不代表平台确认。

```json
{"account_id":"替换为已创建账户ID","observed_at":0,"reserved_cash_cents":200,"entries":[{"key":"替换为扫描商品key","held_units":1,"pending_buy_units":0,"available_units":0,"daily_volume":null}]}
```

observed_at 必须替换为真实填报时间（Unix 秒）；示例 0 会作为过期快照拒绝。最多 2000 个唯一商品，JSON 请求体最大 2 MiB；未来/过期时间、重复 key、预留超过预算均拒绝。

- held_units 包含已挂牌资产；pending_buy_units 仅填尚未计入持有的未决买入，不能重复计数。
- available_units=0 表示无货；null/省略表示未知，绝不转换成 1 件。
- snapshot 启用后未覆盖的候选直接跳过，不假定零持仓。未提供 snapshot 可做匿名预览，并保留缺少仓位的执行阻断说明。
- liquidity_policy 启用后要求对应 entry.daily_volume；不把 MarketItem.volume 推断为日成交量。

流动性参数：participation_bps、low_price_threshold_cents、low_price_penalty_bps、low_price_cap。按日量占比取整；低价时再乘低价系数并应用低价上限；与 max_units_per_item 取小，再扣 held+pending，最后与已知 available_units 取小。零参数是明确的零限额，不用 `or default` 替代。

## 预算语义与历史兼容

`budget_cents = plan.cash_total_cents + plan.reserved_cash_cents + plan.remaining_budget_cents`。

budget 是输入的总现金上限（包含手动填报的已预留）；cash_total 是**本计划拟用现金**，不是实际花费；target_wallet_cents 是**本计划拟新增的估算净钱包价值**，不是现金额度，也不是当前账户钱包余额。资金不足时保留真实拟分配值和 target_met=false，不把计数器改成目标值。

每行新增 max_additional_units；计划保存钱包估算策略、完整手工快照和来源。算法按精确比率的交叉乘积排序，不按已舍入 bps 排序。旧 ratio_greedy_v1 JSON 使用可选字段默认值继续读取，历史数值不重新计算。

前端分别提供“下次扫描的费用参数”和“本次计划的手工快照”；切换费用参数后需要重新扫描。计划展示实际使用的模型及手工来源，改变页面参数不会追改历史结果。

## 历史风险纯函数

PricePoint 含 Unix 秒、带币种价格与可选 volume。按 UTC 日聚合时，只用正成交量记录加权；零量/缺量分开计数，全无有效量的日期 VWAP=None。不补出不存在的交易或均价。

seven_day_risk 仅比较 day 与 day-7 两个有效日；缺少对日不贡献样本。收益以 bps 向下取整，P10 使用升序离散索引 floor((n-1)/10)。样本为空返回 None，0 收益保留 Some(0)。保守价只用 P10 下行部分，再应用显式滑点（bps）；不是预测结果或已验证盈利参数。

输入上限 20000 点，混币种、零价格、重复日曲线或算术溢出返回错误。未来接历史数据时还须核验来源、时区、覆盖、采样周期与实际净值费用。

## 验证

本地测试覆盖：最低费与步长逆推最大可行净值、零费率、u64 边界与币种错配、比例舍入边界、仓位/未决/可买量、缺少日量、未来/过期快照、账户存在性、预留分账，以及 v1 JSON 兼容。

研究样例覆盖零量混入不扭曲 VWAP、P10=0、七条稀疏记录不冒充七自然日。测试不连接平台，不读取真实账户持仓。
