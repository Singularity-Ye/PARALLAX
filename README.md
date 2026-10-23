# 异观 · Parallax

AetherSwap 的 Rust 重制版。**自动化买卖是主线，透明选品和预算计划是入口。**

当前 0.1 已实现并通过本地测试：范围扫描 → 逐条解释 → 预算预览 → SQLite 保存与回看。真实下单、收货、冷却、上架、成交对账尚未迁移；`POST /api/trades` 返回 501。

## 快速运行

主机使用 Rust 1.99.1（`rust-toolchain.toml` 锁定）、MSVC C++ Build Tools 与 Windows SDK。依赖版本记录在 `Cargo.lock`。SQLite 编译进程序。

```powershell
./dev.ps1 run
# http://127.0.0.1:38480
./dev.ps1 test
./dev.ps1 check
./dev.ps1 build   # release 构建
```

也可以直接 `cargo run --locked`。`PARALLAX_PORT` 默认 38480，`PARALLAX_DB` 默认 `data/parallax.db`。服务仅监听环回地址。每个实例使用独立数据库；启动会把旧进程遗留的 running 扫描标记为 interrupted。

页面先用演示数据扫描：13 条来源记录、12 件唯一商品、7 件通过、5 件未通过。然后填写现金预算与选填的钱包目标，生成并保存预算预览。演示数据与真实来源始终明确标记。

## 模块地图

采用单进程、单 Rust crate 的模块化结构。先复用公共模型和流程，出现独立构建或发布需求时再拆 crate。

```text
src/
  domain/             业务事实和纯计算，不依赖网络、数据库或 Web
    selection.rs      选品请求、统一商品、候选理由、覆盖报告
    analysis.rs       金额与预筛规则
    planning.rs       预算、数量、目标和报价时效，输出不可执行的计划预览
  application/        可由网页或未来调度器复用的用例
    mod.rs            扫描任务、取消、计划创建和查询
    scanner.rs        分页、去重、覆盖统计和停止原因
    ports.rs          MarketSource / Repository 接口
  adapters/           外部系统实现接口
    market.rs         演示和 SteamDT 来源，复用 reqwest 连接池
    sqlite.rs         扫描记录和不可覆盖的预算预览快照
  transport/
    http.rs           HTTP/JSON、状态码、静态资源和同源限制
  main.rs             组装依赖、配置、启动
web/
  lib/api.js          与 UI 无关的统一客户端
  lib/api.d.ts        前端字段类型与计量单位
  app.js              页面交互和展示
  index.html          语义化布局
  style.css           视觉样式
```

依赖方向：`transport → application → domain`；`adapters` 实现应用层接口，`main` 组装具体实现。HTTP 层只做协议转换，任务不写在路由中；domain 不读取 SQLite、不请求平台。扫描器的测试可以替换来源，未来计划任务无需复制网页逻辑。

### 新能力放哪里

| 需求 | 复用入口 | 新增的位置 |
| --- | --- | --- |
| 增加选品来源 | MarketSource、统一 MarketItem、扫描与去重 | adapters；补 SourceKind 和组装 |
| 调整筛选策略 | Candidate、RuleResult、整数金额 | domain/analysis |
| 调整预算算法 | ScanReport、PlanRequest、PlanPreview | domain/planning；保留算法版本 |
| 添加定时扫描 | Application.start / cancel / create_plan | 新的调度入口，复用用例 |
| 替换数据库 | Repository | adapters |
| 改页面或换前端框架 | web/lib/api.js + api.d.ts | web |
| 接真实买卖 | 已保存的计划与扫描证据 | 后续 domain 订单状态 + application 执行流程 + 平台适配 |

**计划预览与执行订单分开。** 后续执行流程必须重新核验实价、费用、账户余额、持仓和可购买数量，再创建自己的持久化订单。不能把预览的 `status` 改一下就当成成交。

## 当前算法与边界

- 扫描预算与 UI 展示数量分离；明确页数、条数、去重、拒绝、已收未分析和结束原因。0 不表示无限。
- SteamDT 是只读 BUFF → Steam 余额榜单，非全市场目录；适配代码已实现，当前版本尚未实测实时接口。上游错误保留为错误，不回退到演示数据。
- 钱包估算沿用原项目 `gross / 1.15`；不是精确平台结算。来源成交量统计周期未核实。
- 预算 `ratio_greedy_v1` 按估算折算率升序、商品 key 排序，受现金预算、每种件数、种类数和报价有效期限制。目标按整件分配可能略超；不声称全局最优或考虑市场库存。
- `max_units_per_item` 是本计划上限，尚未包含账户已有持仓。金额 CNY 分，折算率 bps，时间 Unix 秒。
- 计划保存请求参数、来源覆盖、候选报价/理由和跳过原因。历史计划不自动更新或重新执行；无商品可分配时仍保存空预览并说明原因。
- 扫描在开始和结束保存，运行中不提供逐页实时结果。进程中断后本轮尚未提交的数据需要重扫。

## API v1 与前端接手

| 方法 | 路径 | 行为 |
| --- | --- | --- |
| GET | /api/health | 版本与真实能力开关 |
| GET / POST | /api/scans | 最近 30 轮概要 / 创建扫描（202） |
| GET | /api/scans/{id} | 完整结果，running 时每秒轮询 |
| POST | /api/scans/{id}/cancel | JSON `{}`，请求取消 |
| GET / POST | /api/plans | 最近 30 个完整预览 / 创建并保存（201） |
| GET | /api/plans/{id} | 不变的历史预览 |
| POST | /api/trades | 501，未创建订单 |

创建计划的请求示例（scan_id 换成完成的扫描 ID）：

```json
{"scan_id":"...","budget_cents":10000,"target_wallet_cents":null,"max_units_per_item":2,"max_positions":3,"max_quote_age_secs":300}
```

业务错误为 `{"error":"说明"}`；客户端同时处理非 JSON 错误。参数无效 400，JSON 字段类型不匹配 422，记录不存在 404，任务冲突 409。字段含义以 Rust 模型为准，前端类型在 `web/lib/api.d.ts`，改 API 时同步维护。

反重力可直接改 `web/` 布局、CSS、组件和交互，业务计算统一调用后端。采用 React/Vue 时可继续使用同一客户端或按声明生成组件类型；开发服务器把 `/api` 代理到本机 38480，并改写 Host/Origin 保持同源。运行时不开放跨域写入。

当前静态资源由 `include_str!` 嵌入，修改前端后重启 `cargo run` 才会更新。纯视觉工作可用自己的前端预览服务器联调。错误留在当前操作，保留输入；展示必须保留来源、范围、失败原因和 preview_only 标识。

## 下一步

1. 对照旧版接入真实深度、历史稳定性和精确费用，形成统一报价与风险评估。
2. 迁移买入、付款核验、收货匹配、冷却、挂牌和成交对账；引入可恢复订单状态与幂等执行。
3. 收藏、标签、保存范围与前端视觉收口。

知识库：`C:\Users\Yhx06\Documents\Obsidian Vault\02_丹炉间（项目区）\异观（Parallax）`。00 放架构，10 放技术决策，20 放协作与接口，80 按日期堆叠日志，90 保留旧版参考。README 跟代码；架构总稿与阶段记录跟 Obsidian。

## 来源与许可

基于本地 `AetherSwap_audit`（Steam-Alpha，4dcd80d + 本地改动）及 `AetherSwap_reference`（AetherSwap，02552e5 + 本地改动）研究。原工程及其本地改动保留，不迁移账号配置和密钥。

复用 [Axum](https://docs.rs/axum/latest/axum/)、Tokio、[reqwest](https://docs.rs/reqwest/latest/reqwest/) 和 [rusqlite](https://github.com/rusqlite/rusqlite)。Axum 与 Tokio/Tower 直接组合，适合现有异步任务结构；没有进行框架性能排名。项目保留 GPL-3.0-only，见 LICENSE 和 NOTICE；公开分发前仍需整理锁定依赖的完整许可清单。
