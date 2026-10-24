# 压力净卖出估值与策略 v2

2026-10-24。已将原币壁垒/断层参考、七自然日历史收益场景、原币费用和 CNY 净钱包估值接入策略研究。**这是购入计划的退出估值条件，不是完整出售策略、卖单或成交承诺。**

## 计算顺序与复用

提取原 `app/sell_pipeline.py` 的原币费用/换算保护与既有历史风险思路，复用当前 Rust 模块。`domain/exit_valuation.rs` 只组合已有规则，`risk::stress_price` 供原 `seven_day_risk` 和新估值共用；不复制费用、定价、日聚合或汇率公式，不运行旧应用。

1. 从显式、未过期、商品匹配的原币盘口计算壁垒/断层参考价 `G`，沿用 `pricing` 算法。
2. 读取已通过历史覆盖门的 `calendar_p10_v1`，要求足够配对、同一实际币种、相同压力滑点。
3. `S = floor(G × (10000 + min(P10, 0)) × (10000 - slippage) / 100000000)`。P10 是历史收益比例，正值不抬价；滑点只扣一次。**基准是当前原币壁垒参考，不是旧历史绝对价格，也不是已经扣滑点的 conservative_gross。**
4. 分别用显式原币 `FeeSchedule` 从 `G`、`S` 反推可负担净额，然后通过计划保存的有效 FX 折 CNY。没有可负担正净额时，模型净额明确为 0，该规则失败；数据缺失则未知。
5. `cash_cents × 10000 < stressed_net_cny × max_ratio_bps`，使用 u128 交叉乘法，相等拒绝。

手算夹具：CNY 当前卖盘10000分，压价后9999分；P10=-2000bps、滑点300bps，压力毛价7759分。平台费500bps/最低7、发行商费1000bps/最低7、净额步长1时，压力净额6748分、费用1011分。该样例只验证计算，不是实盘参数或收益建议。

## 输入与版本

原 schema_version=1 最多六规则，行为不变。新增 schema_version=2 最多七规则，可在 `history_coverage` 之后、原 CV/趋势/P10 之后追加：

```json
{
  "module":"stress_exit_ratio",
  "max_ratio_bps":9000,
  "parameters":{
    "pricing":{"currency":"CNY","wall_volume_threshold":20,"max_ignore_volume":4,"min_lowest_tier_volume":3,"min_step_minor":1,"min_floor_minor":3,"offset_minor":0},
    "fees":{"currency":"UAH","platform_bps":500,"publisher_bps":1000,"platform_min_minor":7,"publisher_min_minor":7,"increment_minor":1},
    "min_pairs":10,"slippage_bps":300,"max_book_age_secs":300
  }
}
```

费用参数必须使用实际盘口/历史原币；UAH 请求实际返回 TWD 时，不能直接把 UAH 历史或费用贴到 TWD 盘口。历史覆盖门负责当前结束的 UTC 日、样本/缺口、读取有效期与参考日滞后；估值再检查配对数、滑点和原币。盘口沿用 `native_ask` 的来源/时间/空盘/交叉保护，独立有效期1–3600秒。

POST `/api/strategy-evaluations` 新增可选 `order_book_snapshot_ids`，至多100唯一ID；每件一份，名称匹配计划条目。未启用压力规则的策略不得带无用途的盘口；演示计划禁止引用真实盘口/历史。不存在404，错误/重复商品引用400，均不保存研究。

外币估值只使用 `plan.fx` 的显式证据；不存在、过期或旧六币缺UAH时保持未知，需重新生成含外币盘口/有效汇率的计划。当前研究用例仍只有仓储依赖，**不请求平台、凭据或自动刷新FX**。

## 输出与兼容

- v2研究 `algorithm=purchase_research_v2`，新增 `order_books` 完整不可变证据；旧v1算法名保留。
- 成功计算的商品新增 `exit_valuation`，算法 `wall_gap_p10_exit_v1`：含完整 pricing、P10/配对/滑点、stress_base、原币压力毛价、普通/压力费用引用和两种 CNY 净钱包。
- RuleDecision 保留盘口/历史ID、现金价、压力净钱包与阈值；原计划及估值不改。
- 无新表，沿用版本仓储和不可覆盖研究记录。旧JSON缺新字段默认为空/None，序列化省略；旧结果回看不补算。`preview_only=true`、`execution_allowed=false` 固定。

新版本的修订号仍从v1开始；schema版本与修订号不同，不能据修订号判断算法。默认表单数值只是研究例子，费用未做真实结算验证。

## UI复用与验证

策略页复用定价页的组件，见 [UI复用与交接](ui-reuse.md)，没有第二套表单或算法。规则启用时保存schema2，历史覆盖关闭时不能启用该规则；按条目显式选择已有盘口。

新增9项领域/HTTP/SQLite测试，完整 **135通过、0失败、2默认忽略**；fmt、Clippy、JS语法、debug构建通过。覆盖手算、重设基准/一次滑点、正P10不抬价、总滑点/费用后无正净额、UAH/FX/币种不符、严格边界、缺/旧盘口、覆盖门、schema1拒绝新规则、错误/重复/无用途引用、HTTP保存和重开不变。

浏览器核验共享表单启用/重置、两个实例不串值、未保存输入跨页保留、七币选项、原定价页费用计算，以及保存UAH费用的schema2研究草稿和明确的演示unknown记录。正向压力估值用合成夹具验证；用户库没有模拟平台历史或估值成功样本，本轮没有平台行情/账户/交易请求。截图捕获工具本轮不可用，保留实际页面与可复验的操作记录。

剩余：真实乌区原币历史/费用/结算、完整快慢卖/止盈/持仓成本、卖出数量/资产预检、订单/收货/上架/成交对账。不能把“退出估值条件”当成出售执行完成。
