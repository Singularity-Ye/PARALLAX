# 独立商品历史与持有研究

2026-10-25。新增显式市场名称的历史入口与独立持有研究，不要求SteamDT榜单、折算率筛选或预算计划。不是全市场爬取，也不包含自动买入。

## 使用

1. 价格历史页选择“指定市场名称”，输入准确market_hash_name，选择账户和窗口，显式点击读取。也可从库存商品进入。
2. 策略中心保存/选中schema3、objective=holding的策略，选择研究账户。
3. 在“独立持有研究”勾选已保存历史，每商品一份，点击保存；上方预算保持未选择即可。
4. 回看独立记录不会重新请求平台或重算，记录嵌入当时策略版本和历史证据。

只对用户明确输入的一件商品读取；复用Steam身份核验/冷却/凭据门、原历史适配和分析。manual_item=true、scan_id和inventory_snapshot_id互斥；名称1–500字节、无控制字符或首尾空白。请求主机固定，由query参数编码，不接受自定义抓取URL。

## 接口

原POST /api/accounts/{account}/price-histories：

```json
{"manual_item":true,"item_key":"Prisma 2 Case","window_days":90,"slippage_bps":300}
```

新结果identity_binding=manual_market_name_and_request_parameters，仅说明请求参数绑定，不证明当前持有或平台签名确认此商品。旧扫描/库存JSON兼容，旧结果不回写。空历史/错误和币种未知仍按原合同处理。

| 方法 | 路径 | 行为 |
| --- | --- | --- |
| POST | /api/accounts/{account}/holding-research | 201保存不可覆盖独立研究 |
| GET | /api/accounts/{account}/holding-research | 最近10份概要 |
| GET | /api/accounts/{account}/holding-research/{id} | 同账户保存详情，跨账户404 |

创建body：

```json
{"strategy_id":"saved-holding-strategy","revision":1,"history_snapshot_ids":["saved-history-id"]}
```

1–20个唯一快照，同账户且每商品一份，可显式选择扫描、库存或指定名称来源。仅接受有效holding策略；不要求原扫描存在、商品通过预筛或预算分配，也不创建隐藏PlanPreview。所有请求同源、body至多4KiB、响应no-store。

## 架构与证据

历史覆盖/CV/趋势/P10提取为domain/strategy_history共享函数；原计划研究与新入口使用同一计算，测试比较结果一致，不复制一份规则引擎。HoldingResearchService只有仓储依赖，不持有平台客户端或凭据；读取历史和独立规则研究是两次明确操作。

新增holding_research表，只追加完整JSON及账户/时间索引字段，旧库启动兼容创建；strategy_versions及历史快照复用。报告算法independent_holding_research_v1，范围explicit_history_snapshots，无plan字段，preview_only=true、execution_allowed=false、holding_value_established=false。结果ID不属于采购研究表，不能据此创建采购预检/资金预留。

仓位规则需要完整持仓/购买数量，当前独立入口没有这些输入，若启用就保持unknown；如仅研究历史可不启用，但未启用项明确列出。不填零持仓，不推导低估/可买数量/长期收益。

## 验证与边界

新增8项，完整200通过/0失败/2默认忽略；覆盖来源互斥/精确名称、原认证门、独立与计划历史规则一致、库存/指定名称、错账户/重复商品/伪造绑定拒绝、缺失与过期保持未知、范围预算、HTTP同源/不可覆盖/旧库重开。Clippy、fmt、JS语法通过；真实页面证据收口后追加。

页面最多展示该账户最近10份历史供选择，API可显式引用其他已知ID；不是持久化关注列表或全市场目录。连续多日纸面观察、对照组、现金合理价、类别相关风险与样本外有效性仍待做。不会为了验证策略连续抓多轮，也不请求BUFF。

真实页面验收：直接指定Prisma 2 Case读取90天UAH历史成功，响应3089条、窗口内773条，90有效日、83七日配对。保存一份明确标注参数示例/未验证收益的初始历史观察策略（未覆盖旧策略），取消本次不适用的仓位规则，未启用项明确保留。预算保持未选择，勾选此历史后独立研究保存成功；CV约12.58%高于原示例阈值10%，如实未满足，历史覆盖和P10满足，未调参追求通过。本轮只读取这一个商品，没有新扫描、预算、采购或BUFF请求。页面控制台error为空。
