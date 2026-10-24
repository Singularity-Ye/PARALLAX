# 持仓、观察与成本汇总

`domain/holdings`将同一账户的已登记采购批次与**显式选择的库存快照**按资产ID去重。它是研究汇总，`complete=false`固定，不把人工台账、部分库存或遗漏当完整持仓，不自动放行预算/预检。

| 字段 | 语义 |
| --- | --- |
| pending_units | 批次总量减已绑定资产；是未收采购，不是额外扣现金 |
| held_units / listed_units / sold_units | 来自已登记资产状态，互斥 |
| observed_untracked_units | 库存观察可见、尚未归入任何采购批次的数量，成本未知 |
| open_cost_cents | 未收分摊余款 + 持有/已挂单件成本 |
| sold_cost_cents | 已售具体资产的分摊采购成本 |
| reported_wallet_receipts | 按实际币种分别合计的已报净钱包；UAH和CNY不相加 |
| discrepancies | 身份不符、台账已挂/已售但库存可见、完整库存未见台账持有资产等 |

资产在库存和台账中同时出现只计一次。部分库存缺失不等于零；完整观察中未见持有资产只报告冲突，不自动记成交、删除成本或覆盖历史。未知名称不猜商品归属；保护区也保留观察性质，不推导可执行挂牌。

原币Steam钱包是消费余额，不是可提现现金。没有固定0.85变现系数、隐式换汇或跨币种收益差。出售研究中的CNY估值差额有另外的明确假设，见 [sell-policies.md](sell-policies.md)。

调用 `POST /api/accounts/{account}/holdings-preview`：`{"inventory_snapshot_id":null}`仅汇总已登记台账；填写快照ID才合并相应观察。账户与Steam主体不符404，未来观察400。响应保留完整批次/库存来源时间、逐商品数量/成本与警告。生成时间不替代原始观察时间。

本轮未加入资金预留、真实挂牌/未决订单同步或完整持仓认证。原预算的手工PositionSnapshot继续独立；后续要以可信来源补齐在途/挂牌与余额，再显式衔接预算及预检，不能将complete直接改成true。运行：`cargo test --locked --offline ledger`。

浏览器隔离样本验证：收2/未收1/观察未入账1，已绑定资产未重复计数；售出1件后未结666分、已售334分、净钱包UAH43独立显示，库存仍可见该资产时给出对账冲突。当前完整162通过、2默认忽略；平台实测另行进行。
