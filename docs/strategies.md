# 购入策略研究 v1/v2

已实现并本地验证：类型化规则、不可覆盖版本、显式计划/历史引用、逐商品三态解释、研究记录与策略中心页面。研究使用 `purchase_research_v1`，草稿 `schema_version=1`。v2新增可选压力净卖出条件与显式盘口引用，完整合同见 [exit-valuation.md](exit-valuation.md)。没有激活、后台调度或交易动作。

## 继承什么，在哪里复用

原版 `app/strategy_engine.py`、`app/routes/strategies.py` 和 `tests/test_strategies.py` 提供了模块、参数 schema、顺序/依赖、保存与模拟的思路。本版先迁移购入研究子集，不运行 Python/自定义代码，也不复制原模拟接口的预置结果。

- `domain/strategy.rs`：固定模块枚举、参数/依赖校验与纯规则解释。复用 `positions::remaining_units`、计划的费用估值、历史已保存的稳定性/P10和共享 FX 校验。
- `application/strategy.rs`：读取明确版本及证据、账户与身份核对、保存研究。只依赖仓储 ports，不能读取凭据或请求平台。
- `StrategyRepository` / `adapters/sqlite/strategy.rs`：版本比较并追加，研究仅 INSERT。新增两表兼容旧数据库，不修改旧计划/历史格式。
- `transport/strategy.rs`：HTTP/JSON、同源限制、16 KiB 请求上限、no-store。`main` 组装。
- `web/strategies.js`：现有纸感组件与统一客户端，仅编辑参数、选择证据和显示 Rust 结果；换前端无需重写规则。

现有 serde 标签枚举、SQLite 原子写入和规则模块足以覆盖本切片，未引入通用表达式/工作流引擎、统计库或新的认证实现。

## 规则顺序与单位

v1规则共 1–6 项；v2最多7项并可在最后加入stress_exit_ratio（依赖history_coverage）。以下六规则口径不变，必须以下表顺序出现，不可重复。必须包含 `strict_ratio`；三个历史指标必须前置 `history_coverage`。省略可选规则表示该版本不检查该项，页面展示完整版本与规则，不能据此宣称完整交易预检。

| 模块 | 参数与行为 |
| --- | --- |
| strict_ratio | `max_ratio_bps` 1–20000；`cash_cents * 10000 < estimated_wallet_cents * max_ratio_bps`，u128 交叉乘法；相等拒绝，不用显示舍入值比较。复用计划的 CNY 现金/估算净钱包，不重新选择费用模型 |
| position_capacity | 计划内手工仓位必须覆盖该商品且未过期；复用持有+未决扣减、日量/低价限制与可买量。计划数量不能超过剩余额度，手工仓位仍未经完整账户对账 |
| history_coverage | 有效日 2–365、缺失/不可用日上限 0–365、读取有效期 60–86400 秒、最后有效日落后 1–30 日；窗口必须止于本次研究的 UTC 当天（不含当天） |
| sample_cv | basis=`raw` 或 `iqr_cleaned`，CV 上限 1–100000 bps；同时保留原始 CV/IQR 排除证据，少样本未知 |
| trend7 | 最近七自然日，最少点 3–7，相对斜率上下限各在 ±10000 bps/日，R² 下限 0–10000；常量序列 R² 未定义时未知 |
| calendar_p10 | 七自然日配对至少 1–358，收益 P10 下限 −10000–100000 bps；压力滑点 0–10000 必须与已保存历史一致，少配对/不匹配为未知 |

报价有效期是所有研究的基础检查，沿用计划报价观察时间和原有效期，未来/过期为未知。若计划包含 FX，则沿用完整快照及六小时/汇率日期检查；研究不静默刷新汇率、盘口、BUFF 或历史。

历史只接受显式 ID，绑定账户、扫描及计划商品，每件一份；不能跨扫描复用，也不自动挑“最好”的快照。演示计划禁止引用真实平台历史。旧快照缺 `calendar_stability_v1` 时不补算；未知币种、来源/身份标记不匹配、过期或上一结束日的历史保持未知。覆盖不满足时本项失败，后续历史指标未知。

## 状态与不可变证据

每项返回 `pass/fail/unknown`、中文 reason 与 observed 证据。聚合：有 fail 则 fail，其次有 unknown 或空计划则 unknown，全部有证据且满足才 pass。**pass 只表示所选研究规则满足**。

研究保存完整策略版本、原计划（含费用/仓位/盘口/BUFF/FX 证据）以及选中的完整历史快照，回看不读当前行情或重算。`preview_only=true`、`execution_allowed=false` 始终固定，并保留原计划执行阻断说明。不会删掉不通过商品后重分配资金；只复核计划已有条目，不研究扫描中未分配的候选。

策略名称 1–120 UTF-8 字节，说明最多 2000 字节。未知 schema、模块、字段或依赖/顺序错误拒绝保存。没有任意脚本、动作、凭据参数或隐式版本更新。修改请求必须指定 `expected_revision`；SQLite 单语句比较 MAX(revision) 并追加，下游/跨进程并发只有一份成功，冲突 409；旧版本和旧研究不覆盖。研究写入失败不报告已保存。

## API 与最小操作

启动本机服务，打开 `http://127.0.0.1:38480/#strategies`。保存研究规则，选择已有预算及明确账户，再逐商品选择历史并研究。表单默认数字只是未验证的研究示例，不是实盘参数建议。未保存的改动不参与研究；版本提示和结果显示实际使用版本。过期计划需在预算页用新报价重新生成，研究不能刷新其时间。

| 方法 | 路径 | 返回 |
| --- | --- | --- |
| GET / POST | /api/strategies | 最新 50 个策略头 / 创建 v1，201 |
| GET / POST | /api/strategies/{id}/versions | 最近 100 个版本 / 指定 expected_revision 追加新版本，201 |
| GET | /api/strategies/{id}/versions/{revision} | 精确版本；不存在 404 |
| GET / POST | /api/strategy-evaluations | 最近 20 份概要（不展开证据）/ 创建不可变研究，201 |
| GET | /api/strategy-evaluations/{id} | 完整研究及证据，旧结果不重算 |

创建策略的最小请求：

```json
{"schema_version":1,"name":"折算率研究","description":"仅作已保存计划研究","rules":[{"module":"strict_ratio","max_ratio_bps":9000}]}
```

修改版本：`{"expected_revision":1,"draft":{...完整草稿...}}`。研究请求：

```json
{"strategy_id":"saved-strategy-id","revision":1,"plan_id":"saved-plan-id","account_id":null,"history_snapshot_ids":[]}
```

显式历史引用最多 100 份唯一 ID，必须指定账户且与计划账户一致。错账户返回统一 404，引用错商品/扫描或重复商品返回 400，均不保存研究。无历史引用可保存 unknown 解释；这不是正常数据的替代品。

统一 JS 客户端和类型见 `web/lib/api.js` / `api.d.ts`。切页保留 DOM 和未提交输入；版本冲突/读取失败在当前操作反馈。列表刷新失败与已保存成功分开提示。

## 验证与待补

新增 15 项领域/HTTP/SQLite 测试；完整 117 项通过、0 失败、2 项默认忽略。覆盖精确折算率边界、缺失/旧版/过期/未来证据、账户/扫描/商品绑定、CV 口径、趋势未知、配对/滑点、仓位、演示隔离、FX 过期、跨连接版本并发、不可覆盖与重开、跨站/请求上限及未知字段。fmt、Clippy、JS 语法与 debug 构建另行核验。

待迁移：动态阈值/布林带、出售策略及完整模块注册表、导入导出、自动激活/调度、从研究候选重新规划预算、预检/订单/付款核验/持仓账本。真实 SteamDT/历史/BUFF 协议、完整持仓和策略收益仍未验收，本切片没有请求交易平台或创建订单。

## 当前v2增量

压力净卖出估值已作为可选研究条件接入，见 [exit-valuation.md](exit-valuation.md)。schema1原行为不变；schema2新记录algorithm=purchase_research_v2，order_book_snapshot_ids显式选择，结果含order_books和逐商品exit_valuation，旧JSON缺字段兼容。当前完整135通过、2默认忽略；上方117是v1阶段证据。定价/费用表单与证据展示由两页共用，交接见 [ui-reuse.md](ui-reuse.md)。完整卖出策略/订单仍待做。
