# 壁垒／断层挂牌参考 v1

2026-10-24。M07 已实现并本地验证；平台定价效果、完整策略和真实上架尚未验收。复用已保存的 `OrderBookSnapshot`、`PriceLevel`、整数 `Money` 和 `FeeSchedule`，没有新依赖、数据表、平台请求或另一套策略引擎。

## 继承什么

两版 `steam/market_orders.py` 的 `_get_dynamic_thresholds` 与 `compute_smart_list_price` 相同（reference 809/819，audit 848/858）。原 `pipeline_steps.py` 与 `sell_pipeline.py` 将其用于买入前估值与出售定价。本切片先交付同一份纯规则和已有盘口上的研究入口，后续策略引用同一规则；没有自动替换扫描/计划原有最低卖价。

1. 卖盘严格升序、同价已在适配层合并；规则拒绝未排序、重复、零价/零量、超限或交叉盘口，不静默整理脏输入。
2. 当剩余至少两档时，逐档剔除数量 `<= min_lowest_tier_volume` 的最低档。**此步骤按单档判断，没有累计剔除量上限**；不应误解成断层的 max_ignore_volume。始终保留最后一档。
3. 从保留首档累计数量，达到 wall_volume_threshold 截止；深度不足则止于最后可见档，reached_threshold=false，不伪称找到完整墙。
4. 默认参考为保留首价减 min_step。只在墙范围内依次检查相邻档，每次将较低档数量累加到 blocking；`blocking > max_ignore_volume` 则停止，等于仍可检查。价格差严格大于绝对/相对阈值中较大者，才将参考移到上档减 min_step；之后可继续跳档。
5. 最后加 signed offset，再应用 floor。无卖盘返回 null，不用 floor 冒充市场价。

| 较低档价格（CNY） | 绝对阈值（分） | 相对阈值（bps） |
| --- | --- | --- |
| <5 | 10 | 800 |
| [5,20) | 30 | 500 |
| [20,100) | 100 | 300 |
| [100,500) | 500 | 200 |
| ≥500 | 1000 | 150 |

默认墙量20、阻挡量4、低量档阈值3、步长1分、floor3分、offset0。都是继承的研究参数，**不是经平台验证的最小挂牌价格、真实价格步长或盈利参数**。规则使用整数分、bps、u128 交叉乘法、i128 有符号中间价；不继承 float/round 的误差。只接 CNY，不把这些价格分段套到 TWD/USD，也不自动换汇。

有意解释差异：旧实现一旦后续因阻挡量停止，会把理由覆盖为“阻挡量超阈值停”，即使最终价格仍来自之前一次跳档。本版 reason 保留实际选价来源，stopped_by_blocking_volume 单独报告停止原因；价格路径保持继承意图。

## API 与运行

`./dev.ps1 run`，打开 `http://127.0.0.1:38480/#steam-research`，读取/选择已有 **CNY** 盘口，展开“壁垒／断层定价研究”。外币选择会禁用计算；此处“读取盘口”仍会请求平台，而“计算参考价”只读本地。

`POST /api/steam/order-books/{id}/listing-preview`：

```json
{
  "policy": {
    "currency": "CNY", "wall_volume_threshold": 20, "max_ignore_volume": 4,
    "min_lowest_tier_volume": 3, "min_step_minor": 1,
    "min_floor_minor": 3, "offset_minor": 0
  },
  "max_quote_age_secs": 300,
  "fee_schedule": null
}
```

policy 和时效必填；fee_schedule 可省略/null。费用模型沿用 FeeSchedule 的币种、platform_bps、publisher_bps、两个 minimum 和 increment，币种必须 CNY。未提供时净额保持未知；已提供但无法负担正净额时也为 null，request 保留区别。复用已有净额逆推，不复制费用公式。

成功200 + no-store；缺失快照404，业务参数/外币/交叉/未来时间400，JSON类型错误400/422，异源403，body上限2KiB。墙量/步长/下限范围1–10^12，两个数量阈值0–10^12，偏移±10^12；费用金额与步长上限10^12，费用bps0–10000，时效1–86400秒。计算价超10^12拒绝。原快照/扫描/计划不被改写，计算结果当前不单独持久化。

响应为 preview_only、原 snapshot_id/request市场/source/identity_binding、observed_at、computed_at、age_secs、freshness、完整输入、analysis、fee_estimate、warnings。analysis 包含算法 wall_gap_cny_v1、毛价或null、实际选价reason、逐档剔除证据、原始墙索引/累计量/达到标志、各次比较阈值与跳档结果、是否阻挡量停止、是否应用floor。

超过所选时效仍允许历史研究，freshness=historical_only，不能因为重新计算得到当前时间就当作新报价。未超过时效为 within_selected_age，仍不是执行许可。平台返回的深度不保证完整，响应商品身份仍只通过请求参数绑定。

## 验证与剩余

本轮新增7项领域测试、2项应用/HTTP测试，常规 **88通过、0失败、2默认忽略**；fmt、Clippy、JS语法通过。覆盖每个价格段边界/严格等号、空/单档、重复/乱序/交叉、连续低量档、墙量不足/达到、累计阻挡量等于/超过、保留先前跳档、负中间价/偏移/floor/范围、外币/未来、费用币种、历史时效、同源、no-store及缺失404。应用测试证明provider调用0次、原快照时间和价格不变、数量不增加。

未做真实登录、带凭据平台读取、真实挂牌或收益验证。下一步完整历史指标/策略合同；之后才能显式将挂牌参考与历史风险、现金成本、费用、仓位及预检关联。
