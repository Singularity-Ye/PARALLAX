# 基于分摊成本的出售研究 v1

出售策略与购入策略使用不同判断。原版 R 的立即/趋势等待/收益保护/暂停思路保留为类型化研究模式；复用已实现的自然日趋势、壁垒/P10/一次滑点、原币费用与FX，不另造行情算法。**没有调用上架/确认/撤单/成交接口，execution_allowed=false。**

## 输入与资格

账户路径下显式指定采购批次、asset_id、已有v2研究、库存快照以及1–300秒库存期限。批次、库存、研究与Steam主体必须属于同一账户；研究必须含对应商品身份。检查：资产held态、原库存时间、asset/class/instance/商品/普通区/amount=1、已观察可交易/可售标志、最近300秒Steam元数据及历史凭据代次。完整观察中缺失资产阻断，部分观察中缺失保持unknown。不会推算“购入后七天即可卖”。

按当前时间重新计算引用研究的v2退出估值，沿用其明确历史覆盖、盘口和FX有效期。只提取该商品的退出估值与历史趋势，**不把购入strict_ratio失败当出售失败**。没有启用压力规则、证据缺失/过期/覆盖不足时估值未知，不能用缺省跌幅放行。

## 模式与金额

| 模式 | 行为 |
| --- | --- |
| pause | 结果hold，保留缺项/阻断证据，暂停不被包装成可挂牌 |
| immediate | 已知资格与退出证据满足后给review_listing；只可复核条件，不是订单 |
| trend_wait | 有足够点数和R²的斜率严格高于阈值则hold；阈值相等不等待。缺点数/斜率/R²为unknown |
| profit_protect | 普通或压力CNY净钱包对比具体资产分摊成本，含可选趋势等待；收益未达到则hold |

收益保护精确比较 `net_cny * 10000 >= cost_cents * (10000 + min_wallet_return_bps)`，边界包含相等；不先四舍五入显示比例。最低回报范围-10000至100000bps。输出单件成本、当前估值的钱包差额和截断显示回报率；这不是已实现利润、可提现现金或收益保证。

已知身份/状态/时效/会话错误或没有正净额→blocked；资格/估值/策略未知→unknown；策略等待→hold；其余→review_listing。台账仍人工未核验、费用/解锁/权限需实测、执行/订单未接等限制保留在execution_blockers。真实执行始终关闭。

## API与历史

| 方法 | 路径（账户前缀/api/accounts/{account}） |
| --- | --- |
| POST / GET | /sale-research 创建研究（201）/最近20摘要 |
| GET | /sale-research/{id} 完整当时快照 |

```json
{"lot_id":"saved-lot-id","asset_id":"bound-asset-id","evaluation_id":"saved-v2-evaluation-id","inventory_snapshot_id":"saved-inventory-id","max_inventory_age_secs":90,"policy":{"mode":"profit_protect","min_wallet_return_bps":500,"basis":"stressed","trend":null}}
```

新表只INSERT；完整保存所用批次版本、账户/会话上下文、库存、重算证据、参数/原币估值及逐项checks。后续台账/凭据变化不改旧研究；回看不重算。不存在404、语义400、JSON未知字段422、跨站403，16KiB/no-store；GET不泄漏其他账户的研究。

“上架记录”页现含出售研究及历史，真实挂牌列表/确认/撤单仍待接。和预检共用 `evidence-view`逐条解释，和策略研究共用 `exitValuationView`，主题/导航/API/字段助手集中管理；切页保留输入，失败保留原结果。

## 验证与剩余

本组18项新增、完整162通过/0失败/2默认忽略，fmt/Clippy/JS/debug构建通过。覆盖收益包含边界/压力净额、上升/缺R²、已挂/已售/旧观察/代次轮换、部分范围缺失、购入失败不作出售门槛、历史不可覆盖/重开与账户隔离。

隔离浏览器样本：单件成本334分，普通毛价9999分、压力7759分、普通净8696分、压力净6748分；立即模式review_listing，上升趋势hold，已售资产blocked；批次到v6后原研究仍显示v2。截图 `.run/sale-policy-qa.png`，全部为合成证据，没有给正式库造平台数据。尚未完成线上协议、完整持仓/资金、费用校验、动态策略/激活调度、真实订单和自动对账。
