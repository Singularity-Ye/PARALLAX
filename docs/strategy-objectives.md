# 策略研究目标与价格证据 v3

2026-10-25。复用StrategyResearch、PlanPreview、历史规则与现有不可覆盖仓储，不新增API、依赖或表。此切片区分判断含义，不证明策略收益有效。

## 目标与版本

StrategyDraft新增objective：wallet_conversion（缺省）或holding。旧schema1/2仍只接受钱包目标，缺字段按原含义读取与序列化，旧算法不变。新页面保存schema3，默认新建持有观察。

- wallet_conversion：必须以strict_ratio开始，可使用压力净钱包折算率；现金成本/估算净Steam钱包不解释为可提现现金收益。
- holding：仅schema3，必须启用history_coverage，禁止strict_ratio/stress_exit_ratio。复用仓位、历史覆盖、CV、趋势、P10，但规则通过只表示满足观察条件，不建立公允现金价值、低估结论或未来持有收益。

两目标仍复核已保存计划条目；holding并没有绕开上游SteamDT榜单、折算率预筛及预算分配。独立持有选品与更广商品集合仍待接入。不能将本切片说成完整长期投资策略。

## 最小请求

POST /api/strategies：

```json
{"schema_version":3,"objective":"holding","name":"持有观察研究","description":"研究参数示例，未验证收益","rules":[{"module":"history_coverage","min_usable_days":20,"max_missing_days":2,"max_age_secs":3600,"max_reference_lag_days":1}]}
```

其他创建、修订、引用、账户隔离、版本并发检查与策略研究API沿用。持有观察仍使用与计划扫描精确匹配的历史，库存来源历史不混入采购计划。

## 每商品复核说明

新生成研究的items[].review保存objective、conclusion_scope=configured_rules_on_saved_plan_only、现金价格依据、证据时间、可选报价快照ID，以及未启用规则、未评估事项和人工复核清单。

cash_price_basis区分demo、steamdt_reference和buff_visible_sell_snapshot。SteamDT参考价不冒充可成交卖单；已存BUFF快照也不证明当前可买数量/价格。本轮不访问BUFF或其他平台。price_evidence_at沿用保存证据，非本次研究刷新；SteamDT条目可能已合并更保守的现金/Steam两侧时间，不声称其为平台成交时间。

持有价值标志holding_value_established固定false。pass仅聚合已配置规则；未启用规则单独列出，不作为已通过。未评估内容包括实际现金成交、持有期收益、类别/组合相关性、供应事件、样本外效果及上游范围限制。重叠七日配对不作为独立试验。

新算法purchase_research_v3，preview_only=true、execution_allowed=false。持有观察不能进入采购预检或本地资金预留，不将规则观察通过转为采购依据。旧研究缺review按None读取，页面明确旧记录没有保存该清单，不回看时静默重算。新规则评估与旧结果保持不可覆盖。

## 后续

优先补独立持有候选来源/现金成交证据、纸面观察与对照组，再按商品类型和资金占用验证参数；不会用更多指标替代样本外验证。当前六项新增测试覆盖旧JSON/旧结果、目标约束、钱包价值不进入持有规则、缺历史保持未知、来源/未启用项与HTTP持久化。以实际最终测试输出为准。
