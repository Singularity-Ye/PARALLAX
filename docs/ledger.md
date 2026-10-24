# 采购批次与资产事件台账

台账是后续自动收货、挂牌、成交对账共用的业务事实层。目前入口是**人工凭据登记**，`provenance=manual_unverified`、`platform_verified=false`；不会调用任何交易平台。不能把“已登记”当作平台操作成功，也没有导入原项目数据库或凭据。

## 从原版提取的约束

参考 A `app/trade_ledger.py`、R `sell_pipeline.py` / `sync_sold.py`：保留采购到资产、挂牌到成交的关联、持有成本和钱包收入。改进原版的整表替换、名称猜测、固定1.15净额与推算七天解锁：按账户与稳定资产ID绑定、只追加事件、版本检查、保留原币实报净额，不猜实际解锁。

一批采购最多1000件，总现金成本是CNY分，至少每件1分，上限1e12分。采购时间必须非未来；`purchase_ref`在账户内唯一，`client_key`支持相同输入重试。创建结果HTTP200，`replayed`区分首次/重复；重复输入返回**当前投影**，不会再次记成本。相同键不同内容409，同一采购凭据换新键仍409。

## 状态与成本

批次由原始采购凭据、当前资产投影和不可覆盖事件组成。采购未绑定部分是未收货，不会凭名称自动认领观察到的资产。

| 事件 | 前置条件与结果 |
| --- | --- |
| receive | 显式库存快照属于账户/Steam主体且观察不早于采购；明确asset_ids，CS2普通区2、amount=1、商品/class/instance精确有效。可部分收货，剩余保持未收 |
| list | 批次资产held→listed；必须原币毛价、唯一挂牌凭据及非未来时间。仅登记，没有调用上架 |
| cancel_listing | 当前listed且凭据相同→held，保留旧挂牌/撤单事件 |
| sell | 当前listed且凭据相同→sold；实际净钱包与挂牌毛价同币种、正数且不超过毛价，成交凭据唯一 |

资产状态时间单调不减；撤单后不能倒写旧时间。收货不接受堆叠资产拆分，也不接保护区资产。总成本按 `floor(total / quantity)` 分摊，按明确绑定顺序给前 `total % quantity` 件多1分：1000分/3件为334、333、333，部分收货保留未分摊成本。销售的是哪件，就结转该件成本，不用平均浮点数重算。

同一账户asset_id永久唯一归属一个采购批次；挂牌/成交凭据分别唯一。SQLite `BEGIN IMMEDIATE` 在同一事务内检查请求重试、expected_revision、资产/外部凭据占用、追加事件及更新投影；失败整笔回滚。两个进程/连接竞争只会有一个版本推进，事务不会遗留部分资产占用。

事件只保存操作、引用及result_revision；库存引用指向同库既有不可覆盖快照，不为每次事件复制整个批次历史，避免数据随事件数平方增长。新表兼容原数据库，无覆盖/删除事件API。

## API与页面

路径均以 `/api/accounts/{account}` 开始，响应no-store，同源写入，请求64KiB。不存在404、语义400、冲突409、JSON结构/未知字段422、跨站403。

| 方法与后缀 | 用途 |
| --- | --- |
| GET/POST /purchase-lots | 完整批次列表 / 登记采购 |
| GET /purchase-lots/{id} | 当前投影 |
| GET/POST /purchase-lots/{id}/events | 按版本的事件 / 原子追加 |
| POST /holdings-preview | 显式库存与已登记台账汇总，见 [holdings.md](holdings.md) |

```json
{"client_key":"unique-request-id","item_key":"exact-market-hash-name","quantity":3,"cash_cost_cents":1000,"purchased_at":1792800000,"purchase_ref":"purchase-evidence-id"}
```

```json
{"client_key":"unique-event-id","expected_revision":1,"action":{"kind":"receive","inventory_snapshot_id":"saved-inventory-id","asset_ids":["asset-id-1"]}}
```

“持有饰品”页支持采购、批次选择、显式收货绑定、状态凭据、事件及汇总；时间字段按本地时间输入、Unix秒传输。相同输入复用请求键；错误/版本冲突保留输入及原结果，刷新/切页保留勾选。金额业务只在Rust计算，币种选项/原币格式/重试表单助手共用。

读取批次目前最多10000份，超过会明确失败，不静默截断成完整台账；尚未做分页、凭据纠错/冲销、真实BUFF订单/Steam offer/挂牌/成交接入。采购“未收”是已登记采购义务，不是另一次可用余额预留，不接预检放行或自动覆盖预算手工仓位。

## 验证

本组共新增18项测试，完整162通过/0失败/2默认忽略，fmt/Clippy/JS语法/debug构建通过。包括跨连接版本竞争、收货占用部分回滚、请求与外部凭据防重、成本守恒、账户隔离、原币净钱包、重开及出售研究不变。

独立 `.run/asset-qa.db`、38481端口完成浏览器流程：1000分/3件、收2件、挂牌UAH5000、拒绝重复挂牌、撤单、以新凭据重挂、成交实报UAH4300；v1→v6，未结666分/已售334分。截图 `.run/holdings-qa.png`。隔离实例验收后关闭；正式库未加入合成账户/采购/资产，无平台请求。真实平台结果仍待独立验收。
