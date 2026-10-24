# Parallax

GitHub：<https://github.com/Singularity-Ye/PARALLAX>。项目采用 GPL-3.0-only，保留原项目来源说明；见 [LICENSE](LICENSE) 与 [NOTICE](NOTICE)。

AetherSwap 的 Rust 重制版。**自动化买卖是主线，透明选品和预算计划是入口。**

界面按原项目的左侧导航与功能页组织，已建立仪表盘、选品、预算、库存、记录、研究及账号等独立视图；待迁移入口明确标注。暖白纸感、粗黑字与少量荧光绿已由用户确认为初版 UI，后续按需求精修。页面与复用合同见 [分页工作台](docs/frontend.md)。

当前 0.1 已实现并通过本地测试：范围扫描 → 逐条解释 → 预算/策略研究 → 保存证据预检，以及人工采购台账、精确收货绑定、持仓成本汇总与出售研究。自动下单/收货/冷却/上架/成交对账尚未接通；`POST /api/trades` 返回 501。

金额与预览已升级：可配置整数费用模型、账户手动仓位快照及持仓/预留/可买量约束接入扫描与计划。自然日 VWAP/P10 已接价格历史研究用例；真实历史协议尚未验收，不自动加入扫描/预算。合同见 [估值与仓位 v2](docs/valuation.md) 与 [历史研究](docs/price-history.md)。

账户与会话支持创建账户、手动导入、只读身份验证、脱敏查询和本地停用；已新增 Steam/BUFF 独立浏览器登录与人工恢复，复用相同凭据后端。用户在官方页面完成密码/Guard/挑战，返回本页完成核验后才保存；密码/Guard 自动登录和后台续期待做。真实平台登录尚未验收，不把夹具或空白窗口测试当作平台验证。合同与选型见 [浏览器登录](docs/browser-login.md)。

已接 Steam 公开买卖盘口快照、原币深度与价差、SQLite 历史，以及预算引用。默认人民币显示，支持 CNY/USD/EUR/GBP/HKD/TWD/UAH 自动换算；引用外币盘口时按有效汇率折合人民币，原币和完整汇率快照随计划保存。合同与边界见 [盘口研究](docs/orderbook.md)。

壁垒／断层挂牌定价已接保存的原币盘口：共享汇率模块自动比较人民币策略阈值，返回原币挂牌参考、所选币种估值、逐档解释和原币费用估算。按需获取 Frankfurter 日参考汇率，六小时缓存并保存来源/日期；缺失/过期不猜固定值。仅研究预览，不刷新盘口时间或创建上架。汇率边界见 [多币种](docs/fx.md)。见 [定价合同](docs/pricing.md)。

BUFF 现金端已接原协议只读适配、账户/凭据代次绑定、首屏快照及预算引用；与登录/挑战/冷却共用认证门。只按最低价可见卖单数限量，空盘不退回榜单价。**该切片仅通过合成夹具，真实 BUFF 登录、卖单字段和 CNY 单位尚未验收。** 合同见 [现金端报价](docs/buff-quotes.md)。

Steam CS2 库存观察已接双区分页、账户认证门、局部范围与不可覆盖快照；跨区失败不保存新快照，不把缺失或部分库存当零持仓。解锁时间未知，暂不覆盖手工仓位。**真实库存协议未验收**。合同见 [库存观察](docs/inventory.md)。

Steam 价格历史已接认证只读适配、扫描候选与账户绑定、UTC 日线/P10及不可覆盖研究快照；新增原始/清洗后CV、IQR证据、完整自然日EMA7/30、趋势/R²和历史区间位置。缺数据保持未知，原P10保留极端价，不刷新预算或自动放行交易。**真实历史协议、完整策略与收益仍待验收。** 见 [历史研究](docs/price-history.md) 与 [波动/趋势口径](docs/stability.md)。

购入策略研究 v1/v2 已打通保存版本、显式引用计划/历史、逐商品规则解释和不可覆盖记录。支持严格折算率、仓位额度、历史覆盖、CV、七日趋势及 P10；缺失/过期为未知，旧版本和旧研究不重算。策略中心沿用初版 UI，研究结果始终 `execution_allowed=false`；未接出售策略、激活或订单。见 [策略合同](docs/strategies.md)。v2可选压力净卖出条件，复用当前壁垒参考、历史跌幅/一次滑点、原币费用和已保存FX；不是完整出售执行。见 [退出估值](docs/exit-valuation.md)。

UAH已补齐：可请求乌币盘口、显示乌币折合价、解析明确乌币历史，原币费用/预算/策略证据复用现有模块；旧六币种快照兼容保留。新增9项测试，完整126通过、2默认忽略。公开UAH汇率和页面换算已实测，真实乌区账户协议/费用仍待验收。见 [UAH合同与验证](docs/uah.md)。

UI复用已进一步收拢：两个页面共用币种选项、定价/原币费用编辑器与剔除/断层证据展示；主题/样式/页面元数据/API继续分层。反重力接手先看 [组件清单与边界](docs/ui-reuse.md)，不在各页复制组件后分别美化。当前新增9项，完整135通过、2默认忽略；实际平台/订单边界不变。

交易前**保存证据检查 v1**已接：明确选择研究、账户、商品与数量，重算原策略、检查报价时效/凭据代次/可见量及访问限制，保存不可覆盖结果。策略中心保留失败输入与记录；余额、完整持仓、真实费用和订单等缺项明确为unknown。没有签发交易令牌或创建订单。新增9项，完整144通过、0失败、2默认忽略；合同见 [预检](docs/preflight.md)。

本轮连续补齐三块业务：**采购批次与状态事件、持仓/成本汇总、基于分摊成本的出售策略研究**。人工凭据支持幂等重试、版本冲突及跨批次资产/外部凭据防重；收货成本整数分摊并保留余分，UAH等实际净钱包按币种单列。立即/趋势等待/收益保护/暂停复用现有v2退出估值，历史不随台账更新而改写。当前仍为未核验台账与研究，不发平台交易请求。见 [台账](docs/ledger.md)、[持仓](docs/holdings.md)、[出售策略](docs/sell-policies.md)。本组新增18项，完整 **162通过、0失败、2默认忽略**，页面全流程使用隔离数据库通过。

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

浏览器登录复用本机 Chrome/Edge，不额外下载浏览器。可用 `PARALLAX_BROWSER` 指定可执行文件。每次使用独立临时 profile，不连接个人浏览器；一次一个窗口，十分钟过期，失败保留窗口以便重试。刷新页面可找回本进程的待处理流程；应用重启需重新发起。

页面先用演示数据扫描：13 条来源记录、12 件唯一商品、7 件通过、5 件未通过。然后填写现金预算与选填的钱包目标，生成并保存预算预览。演示数据与真实来源始终明确标记。

## 模块地图

采用单进程、单 Rust crate 的模块化结构。先复用公共模型和流程，出现独立构建或发布需求时再拆 crate。

```text
src/
  domain/             业务事实和纯计算，不依赖网络、数据库或 Web
    accounts.rs       账户身份、凭据校验和脱敏会话模型
    preflight.rs      选择绑定、证据重算、缺项与不可执行检查记录
    ledger.rs         采购凭据、资产归属、分摊成本及状态事件
    holdings.rs       台账/库存去重、成本、原币钱包和冲突汇总
    sale_research.rs  立即/趋势/收益保护/暂停出售研究
    orderbook.rs      原币盘口、深度摘要和 CNY 估值约束
    pricing.rs        可解释的 CNY 壁垒/断层挂牌参考规则
    buff_quotes.rs    账户绑定的首屏现金报价与可见数量
    inventory.rs      CS2 资产身份、数量和分页范围快照
    history.rs        价格历史样本、范围、日线/P10 与独立研究快照
    selection.rs      选品请求、统一商品、候选理由、覆盖报告
    analysis.rs       费用估算、整数比例与预筛规则
    money.rs          币种金额、费率/最低费用与步长逆推
    positions.rs      手工仓位、未决量、流动性与可买量限额
    risk.rs           可复用日聚合与七自然日风险纯计算
    stability.rs      原币日价描述指标，独立于策略阈值
    strategy.rs       类型化购入规则、显式依赖和三态证据解释
    exit_valuation.rs 当前壁垒参考×历史压力场景、原币费用与CNY净额组合
    planning.rs       预算、数量、目标和报价时效，输出不可执行的计划预览
  application/        可由网页或未来调度器复用的用例
    accounts.rs       导入、验证、停用与核验后的凭据替换
    browser_login.rs  独立登录窗口、账户/代次绑定、取消与过期
    orderbook.rs      只读盘口获取、证据保存与历史查询
    buff_quotes.rs    扫描候选映射、共享认证门与账户报价历史
    inventory.rs      双区分页、截断/冲突处理及账户快照历史
    accounts_inventory.rs 复用 Accounts 门与凭据的库存读取
    accounts_steam.rs 库存/历史共用 Steam 凭据访问与错误处理
    history.rs        当前真实候选的账户价格历史研究与持久化
    strategy.rs       版本化购入研究，仓储取证、身份绑定与结果保存
    mod.rs            扫描任务、取消、计划创建和查询
    scanner.rs        分页、去重、覆盖统计和停止原因
    ports.rs          行情、仓储、凭据、验证与浏览器端口
  adapters/           外部系统实现接口
    browser_auth.rs   本机 Chrome/Edge 的独立临时登录窗口
    credentials.rs    Windows 原生凭据后端
    session.rs        Steam/BUFF 只读身份核验
    steam_orderbook.rs 匿名 Steam queryAction 盘口解析
    buff_quotes.rs    BUFF 卖单 GET、整数价格和错误/轮换分类
    steam_inventory.rs Steam 双区库存 GET、精确实例匹配和有限解析
    steam_history.rs  价格历史 GET、精确小数和时区解析
    steam_read.rs     库存/历史共用的状态、重定向与 Cookie 分类
    market.rs         演示和 SteamDT 来源，复用 reqwest 连接池
    sqlite.rs         扫描记录和不可覆盖的预算预览快照
  transport/
    http.rs           HTTP/JSON、状态码、静态资源和同源限制
    accounts.rs       账户与手动会话路由
    browser_login.rs  浏览器登录/人工恢复路由
    orderbook.rs      盘口快照路由
    buff_quotes.rs    账户 BUFF 报价路由
    inventory.rs      账户库存快照路由
    history.rs        账户价格历史研究路由
    strategy.rs       策略版本、并发冲突与研究预览路由
  main.rs             组装依赖、配置、启动
web/
  shell.js            左侧导航、独立视图与原生 hash 前进后退
  lib/pages.js        原版功能映射、分组与页面状态元数据
  lib/api.js          与 UI 无关的统一客户端
  lib/api.d.ts        前端字段类型与计量单位
  lib/currencies.js   七币种选项的唯一UI定义
  lib/pricing-fields.js 共用定价/费用编辑器，实例状态独立
  lib/pricing-view.js 共用定价证据与退出估值展示
  app.js              页面交互和展示
  strategies.js       购入规则编辑、版本与证据研究；不在 JS 计算策略
  index.html          语义化布局
  style.css           引入主题与组件样式
  theme.css           纸底、字体、颜色和纹理 tokens
  components.css      导航、页面与业务组件样式
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
- 钱包估算支持原版 `gross / 1.15`（默认）或用户自填的费率/最低费/步长模型，两者均未验证真实平台结算。来源成交量周期未核实，不作为已知日量。
- 新计划 `ratio_greedy_v2` 按精确估算折算率排序，受现金、种类、时效和可选手工仓位约束。目标按整件分配可能略超；不声称全局最优。历史 v1 计划原样保留。
- 未提供快照时 `max_units_per_item` 仅限制本计划；提供快照时扣除已持有/未决数量，并应用已填可买量及可选日量限制。账户需存在，来源始终标为手工未核验。现金预算包含已填预留，钱包目标指新增估算净值。
- 计划保存请求参数、来源覆盖、候选报价/理由和跳过原因。历史计划不自动更新或重新执行；无商品可分配时仍保存空预览并说明原因。
- 扫描在开始和结束保存，运行中不提供逐页实时结果。进程中断后本轮尚未提交的数据需要重扫。

## API v1 与前端接手

账户与登录态接口、凭据存储与验证边界见 [账户会话合同](docs/accounts.md) 和 [浏览器登录合同](docs/browser-login.md)。前端仍统一复用 `web/lib/api.js` 与 `api.d.ts`；账户 UI 位于 `web/accounts.js`，与扫描和计划用例独立。当前只补必要操作入口；业务打磨后再交反重力统一美化。

| 方法 | 路径 | 行为 |
| --- | --- | --- |
| GET | /api/health | 版本与真实能力开关 |
| GET / POST | /api/scans | 最近 30 轮概要 / 创建扫描（202） |
| GET | /api/scans/{id} | 完整结果，running 时每秒轮询 |
| POST | /api/scans/{id}/cancel | JSON `{}`，请求取消 |
| GET / POST | /api/plans | 最近 30 个完整预览 / 创建并保存（201） |
| GET | /api/plans/{id} | 不变的历史预览 |
| GET / POST | /api/steam/order-books | 最近十份盘口快照 / 匿名读取并保存 |
| GET | /api/steam/order-books/{id} | 盘口历史；不刷新平台 |
| GET / POST | /api/accounts/{id}/buff-quotes | 账户最近十份报价 / 读取当前真实候选并保存 |
| GET | /api/accounts/{id}/buff-quotes/{quote} | 账户绑定的历史报价 |
| GET / POST | /api/accounts/{id}/inventories | 最近十份库存快照 / 显式双区分页读取 |
| GET | /api/accounts/{id}/inventories/{snapshot} | 账户绑定的库存历史，非当前可出售承诺 |
| GET / POST | /api/accounts/{id}/price-histories | 最近十份价格历史研究 / 读取已完成真实扫描候选 |
| GET | /api/accounts/{id}/price-histories/{snapshot} | 保存时的日线/P10；未重新读取或刷新预算 |
| GET / POST | /api/strategies | 策略最新版本 / 保存 v1 |
| GET / POST | /api/strategies/{id}/versions | 版本列表 / expected_revision 并发校验后追加 |
| GET | /api/strategies/{id}/versions/{revision} | 精确不可覆盖版本 |
| GET / POST | /api/strategy-evaluations | 最近研究概要 / 创建仅研究的预览 |
| GET | /api/strategy-evaluations/{id} | 当时规则、计划、历史与解释 |
| GET / POST | /api/preflights | 最近检查摘要 / 保存所选证据的检查（不授权交易） |
| GET | /api/preflights/{id} | 原研究、重算、账户代次、选择与缺项快照 |
| GET / POST | /api/accounts/{account}/purchase-lots | 采购批次 / 幂等登记人工采购凭据 |
| GET | /api/accounts/{account}/purchase-lots/{id} | 当前批次版本、成本与资产 |
| GET / POST | /api/accounts/{account}/purchase-lots/{id}/events | 不可覆盖事件 / 原子版本追加 |
| POST | /api/accounts/{account}/holdings-preview | 显式库存与已登记持仓/成本汇总（不完整） |
| GET / POST | /api/accounts/{account}/sale-research | 出售研究摘要 / 保存研究 |
| GET | /api/accounts/{account}/sale-research/{id} | 当时批次、成本、参数及完整出售证据 |
| POST | /api/trades | 501，未创建订单 |

创建计划的请求示例（scan_id 换成完成的扫描 ID）：

```json
{"scan_id":"...","budget_cents":10000,"target_wallet_cents":null,"max_units_per_item":2,"max_positions":3,"max_quote_age_secs":300}
```

业务错误为 `{"error":"说明"}`；客户端同时处理非 JSON 错误。参数无效 400，JSON 字段类型不匹配 422，记录不存在 404，任务冲突 409。字段含义以 Rust 模型为准，前端类型在 `web/lib/api.d.ts`，改 API 时同步维护。

反重力可直接改 `web/` 布局、CSS、组件和交互，业务计算统一调用后端。采用 React/Vue 时可继续使用同一客户端或按声明生成组件类型；开发服务器把 `/api` 代理到本机 38480，并改写 Host/Origin 保持同源。运行时不开放跨域写入。

当前静态资源由 `include_str!` 嵌入，修改前端后重启 `cargo run` 才会更新。纯视觉工作可用自己的前端预览服务器联调。错误留在当前操作，保留输入；展示必须保留来源、范围、失败原因和 preview_only 标识。

2026-10-24 购入研究切片：新增 15 项测试，完整 **117 通过、0 失败、2 默认忽略**；fmt、Clippy、JS 语法和 debug 构建通过。浏览器已核验规则保存、已有演示计划研究/unknown、历史记录回看和切页保留输入。仅新增一份明确标记的演示研究与一版研究草稿，没有真实平台操作。

## 下一步

完整范围见 [原项目能力与迁移地图](docs/migration-map.md)，逐文件证据见 [源码盘点](docs/source-inventory.md)。已登记两版 337 个文件并静态解析 225 个 Python 文件；这是盘点覆盖，尚未达到行为迁移覆盖。

当前按依赖推进真实只读数据、策略与交易主链；每个切片先核对对应继承合同，再实现和验收。已提取 24 组业务设计并记录 [核心设计审查与原实现局限](docs/audit/design-review.md)；8 项离线观察可用 `python tools/legacy_contract_probes.py` 复现，不执行旧应用初始化或访问交易平台。原完整测试套件与真实协议尚未验收。

1. 验收已实现的独立浏览器登录/人工恢复；后续补地区币种、Guard/自动登录与后台续期。
2. 迁移精确费用、盘口、历史稳定性和持仓限额，再接到现有扫描与预算模型。
3. 迁移持久化订单、预检、付款核验、收货、冷却、挂牌和成交对账，保留两版互补的安全保护与结果账本。
4. 扩展购入研究为完整买卖策略、激活/调度，再按迁移地图补齐后台任务/通知、游戏折扣、礼物等；前端继续复用统一 API。

重建本地静态索引：`python tools/source_inventory.py`（仅工程审计工具；Rust 应用运行不依赖 Python）。

知识库中按项目主页管理架构、技术决策、协作接口、开发日志与旧版参考；代码仓库只使用 **Parallax** 作为项目名称。README 跟代码；架构总稿与阶段记录跟 Obsidian。

## 来源与许可

基于本地 `AetherSwap_audit`（Steam-Alpha，4dcd80d + 本地改动）及 `AetherSwap_reference`（AetherSwap，02552e5 + 本地改动）研究。原工程及其本地改动保留，不迁移账号配置和密钥。

复用 [Axum](https://docs.rs/axum/latest/axum/)、Tokio、[reqwest](https://docs.rs/reqwest/latest/reqwest/) 和 [rusqlite](https://github.com/rusqlite/rusqlite)。Axum 与 Tokio/Tower 直接组合，适合现有异步任务结构；没有进行框架性能排名。项目保留 GPL-3.0-only，见 LICENSE 和 NOTICE；公开分发前仍需整理锁定依赖的完整许可清单。
