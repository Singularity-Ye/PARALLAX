# 高密度台账与仪表盘数据契约

2026-10-27。配合高密度布局，只补读取投影，不变更采购/成交事实、数据库格式或执行权限；前端排版不需要等待真实平台交易才能开工。domain/ledger_view基于现有Holdings和PurchaseLot生成读模型，application/ledger复用holdings，transport只转换HTTP。

## 两个账户级入口

- GET `/api/accounts/{account}/ledger/summary`：仪表盘账户范围内汇总。
- GET `/api/accounts/{account}/ledger/rows?offset=0&limit=50`：明细，limit 1–200，offset 0–10000000。
- JS：`parallax.ledgerSummary(account)`、`parallax.ledgerRows(account,{offset,limit})`。类型见api.d.ts的LedgerSummary/LedgerRows/LedgerRow/LedgerMetric。
- 不新增无账户 `/api/ledger/summary`，不将财务数据挂入health。账户不存在404、语义分页错误400、Query解析错误400；读取失败沿用500。响应no-store。
- 请求仅访问本地已登记台账，不发平台/汇率请求、不更新记录；未自动引用库存。`scope=account_reported_ledger_only`，`complete=false`。不是全账户余额/完整持仓。

## 四格KPI及口径

| 蓝图位置 | 字段 | 页面应显示 |
| --- | --- | --- |
| 购买 | reported_purchase_cost | 已登记采购投入；CNY整批总成本累计，含尚未收货，不能称平台已核验支出 |
| 出售 | reported_sale_net_wallet | 实报出售净钱包，按原币数组分列；空数组是没有登记，不是平台钱包余额为0 |
| 净收益 | total_cash_profit | 当前value=null，用reason解释；没有可提现现金结算，不能从Steam钱包减CNY采购成本 |
| 实际折算率 | actual_discount_bps | 当前value=null；缺少同口径结算/汇率，不用所有批次成本除以已售钱包 |

额外：open_purchase_cost为未结采购成本（含未收/持有/已挂），sold_purchase_cost为已售资产结转成本。counts含lots/pending_units/held_units/listed_units/sold_units。generated_at是本次生成时间，不是市场报价时间；汇总固定platform_verified=false，警告必须保留。空账本采购登记合计可以是0，但净收益/折算率仍未知，不能渲染绿色0收益。现阶段无趋势对比证据，KPI不得凭动画伪造红绿趋势。

## 高密度明细映射

每条已收资产一行quantity=1，未收部分按批次汇为一行，asset_id=null、stage=pending_receipt。row_id稳定用于选择，lot_id/lot_revision供已有事件接口使用。pending行purchase_cost是剩余全部数量总成本，前端不要标成单件价。按purchased_at倒序、row_id升序；返回total/offset/limit/next_offset，超末页为空。

| 表格列 | 对应字段与说明 |
| --- | --- |
| 时间 | purchased_at；状态发生时间另用state_observed_at |
| 物品 | item_key，准确市场名 |
| assetid | asset_id；未收行显示未绑定，不编造ID |
| 数量/采购成本 | quantity / purchase_cost（整行数量的CNY成本） |
| 购入市场价 | purchase_market_price：当前未知，不拿采购成本冒充报价 |
| 现市场价 | current_market_price：当前未知，不用日线或挂牌价冒充现价 |
| 税后/实报净钱包 | sale_net_wallet：只有已登记成交可得，显示实报成交净钱包；未成交不是税后估值 |
| 挂牌参考 | reported_listing_gross：已有挂牌登记的毛价，单独标明，不当实时价 |
| 折算比率 | actual_discount_bps：value=null并带reason |
| 变现收益 | cash_profit：value=null并带reason |
| 自用收益 | self_use_profit：value=null并带reason |
| 市场变动 | market_change：value=null并带reason |
| 操作 | 复用现有purchase-lots/{id}/events；未收聚合行不能执行资产级操作 |

缺失指标统一 `{value:null,reason:"…"}`，0若有则是有效值；不得在UI用 `value || 0`。Money为原币最小单位，CNY默认分，保留UAH，不直接替换币种标签。provenance/platform_verified随行保留；人工登记不改标平台验证。

## 分页与容量边界

当前复用既有单次SQLite批次投影读取（至多10000批次，超过明确失败，不截断成完整总账），在领域读模型上分页，不宣称数据库级分页。每次summary/rows各读一个台账集合，二者不是同一跨请求事务；分页期间发生写入须刷新，当前不是冻结游标快照。没有为了表格增加平行缓存或统计表，规模确需扩大时再扩仓储读模型。

## 不随本轮实现的行为

- 不新增批量出售、挂牌、删除或清空端点。已有版本/幂等/资产唯一性不能被批量表格绕过。
- 多选首先用于本地选择/研究；真实批量指令另定逐项结果、部分失败、版本和幂等合同。台账历史不能直接DELETE。
- 扫描结果沿用原Candidate.item（cash_cents、steam_gross_cents、volume、observed_at）及valuation/ratio_bps/rules；扫描商品没有assetid/真实采购成本，不为了统一13列填假值。
- 当前无完整钱包余额、Trade Offer/待支付平台告警、实时事件流或全市场扫描；相应位置显示未接入，不以静态跑马灯冒充实时流。

## 验证与交付

测试使用合成批次与内存SQLite，涵盖1000分/3件的334/333/333成本、部分收货、UAH/CNY出售分列、未知收益、稳定分页、缺失/其它账户隔离与GET不写事件。没有真实平台请求。最终测试/构建状态随本轮日志记录；接口完成不等于前端已接线或服务已更新。

本轮最终门：完整离线224通过、0失败、2忽略，Clippy/fmt/JS检查通过。通过内存HTTP集成验证，不发平台请求；没有部署或重启共享服务，前端接线/真实数据看板待统一验收。
