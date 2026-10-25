# Parallax Steam 库存观察

状态：账户绑定的只读双区分页、范围说明、不可覆盖快照和页面已实现；**本地合成夹具通过；2026-10-25 单个真实账户登录及双区一页读取通过，真实多页与资产变化仍待验收**。这不是持仓对账、收货确认、出售授权或自动上架。

## 继承与复用

来源是旧版 `steam/inventory.py:_fetch_full_context/fetch_cs2_inventory` 与 `app/inventory_cs2.py`：账户 CS2 库存区 2/16、每页 75、start_assetid、资产与描述关联、交易/市场标志。原版允许只读成功一个区时合并返回，也会按 instance=0 回退和跳过无法关联的资产；重制明确拒绝区读取失败和错实例，不静默当完整库存。

本轮先检查现有 Accounts、reqwest、SQLite 和只读报价端口。参考了 [node-steamcommunity 的库存实现](https://github.com/DoctorMcKay/node-steamcommunity/blob/master/components/users.js)（[MIT](https://github.com/DoctorMcKay/node-steamcommunity/blob/master/LICENSE)，仍有近期发布）作为协议交叉核对：完整 Node 社区客户端需要另一套会话、运行时和桥接，当前切片没有引入。选择适配现有 reqwest，不新增依赖、不复制第三方实现。后续更广的交易协议再评估成熟 SDK，不让这一页读取引入第二套认证。

[Steam 官方交易保护说明](https://help.steampowered.com/en/faqs/view/365F-4BEE-2AE2-7BDD)解释了 CS2 交易后保护窗口；它不提供这里所用社区 JSON 接口的完整协议合同，也不能替代真实样本。区 16 的使用仍来自旧项目，待平台核验。**不从七天规则推算每件资产的解锁时刻。**

## 分层与账户合同

- `domain/inventory`：资产身份、整数数量、页校验、范围与快照；不使用网络、凭据或 SQLite。
- `application/inventory`：双区依次分页、预算、重复/冲突拒绝、保存与账户历史。新端口 InventoryProvider / InventoryRepository；SQLite 只新增 `inventory_snapshots` 表。
- `application/accounts_inventory`：通过共用 `accounts_steam` 获取现有 Accounts 门与凭据访问，整轮与导入、验证、停用、浏览器恢复互斥；价格历史也复用同一访问规则，没有另建会话状态机。
- `adapters/steam_inventory`：固定主机 GET、有限响应和平台字段转换；`transport/inventory` 转换 HTTP。前端复用统一客户端和声明文件。

读取前要求 Steam 会话 valid，subject 等于账户 Steam ID，checked_at 在过去 300 秒内，且 session.retry_at 与全应用库存冷却均已过。凭据再次检查 Steam 身份，仅在门内传给适配器，不写入快照。新导入的 unverified 不能读库存。独立浏览器或手动导入使用同一现有流程。

同一门覆盖两个区的全部请求和分页间隔；替换/停用需要等待本轮读完。网络返回后再保存不可变历史，快照代次表示当时使用的凭据，不代表查询时仍有效。单进程门不提供跨进程认证事务，库存也不是执行授权。

## HTTP 与运行

运行 `./dev.ps1 run`，打开 `http://127.0.0.1:38480/#inventory`。创建账户 → 获取并在线核验 Steam 登录态 → 等待会话冷却 → 选每区页数 → 显式读取。不进行自动登录、重试、后台轮询、收货或上架。

| 方法 | 路径 | 行为 |
| --- | --- | --- |
| POST | /api/accounts/{account}/inventories | `{"max_pages_per_context":4}`；201 保存快照 |
| GET | /api/accounts/{account}/inventories | 此账户最近十份完整 JSON 快照 |
| GET | /api/accounts/{account}/inventories/{id} | 历史详情，跨账户或缺失 404 |

请求默认 4 页/区，只接 1–10 页，每页 75 个资产；固定 app=730，区 2 和 16，总共最多 20 个 GET、1500 个资产。body 上限 2 KiB，同源写检查，所有响应 no-store。无额外配置或依赖，沿用 PARALLAX_PORT / PARALLAX_DB / 原生凭据后端。

错误：输入 400/422；记录不存在 404；未核验/失效/凭据轮换/分页冲突 409；限流 429 与绝对 retry_at；拒绝、网络、格式问题 502。错误均为静态说明，不回显平台原文、Cookie 或 UA。403 可能是库存隐私限制，不据此把有效身份改为失效。401/同主机登录跳转才标记 invalid；认证 Cookie 变化标为 unverified 并换代次，需重新获取核验。网络/格式失败不刷新身份核验时刻。

## 分页、身份与范围

固定 `https://steamcommunity.com/inventory/{account.steam_id}/730/{2|16}`，参数 l=english、count=75、preserve_bbcode=1、raw_asset_properties=1；后续页只用响应的 last_assetid。TLS 开启，禁止重定向、不自动重试；每次 15 秒超时、响应最多 2 MiB，分页请求至少相隔一秒（包括区切换）。请求前持久化两秒库存节流；429 冷却最少 300 秒、最多七天，重启/替换凭据/新增账户不能清除。

`success=1` 和明确的 total_inventory_count 必须存在；只有总数为零且没有后续页时，缺失 assets 才可解释为空区。资产 app/context 必须等于请求；ID 为规范十进制字符串；amount 为正整数，不把资产条目数当成件数。本适配暂按资产条目数核对 total_inventory_count，这一计数口径仍需真实样本验收；不符合时明确拒绝，不猜测数量。

描述精确按 `(classid,instanceid)` 关联，不按名称挑第一个、不回退实例；找不到描述失败，缺市场名称则保留资产但名称为 null。资产在本页/跨页/跨区重复、游标无效或循环、区内总数变化、读完的条目数与报告不符均拒绝保存本次快照。不会把另一区的成功当作本次成功。

达到页数预算仍保存明确的局部快照，coverage 分别记录区、页数、报告总数、已见条目数、exhausted 与 stop_reason。只有两区各自 exhausted 才说明本轮分页读完；**它仍是分时观察，不保证读取期间没有总数不变的资产替换，也不包含挂牌、未决收货或采购**。失败时旧快照保留，明确为历史，不能作为新刷新结果。

## 冷却与仓位的边界

快照保留资产 app/context/asset/class/instance、amount、市场名、平台报告的 tradable/marketable、owner_descriptions 是否存在限制说明。不持久化描述 HTML、操作链接或原始响应。区 16 或存在 owner 说明均不报告可转移；区 2 的可转移展示也只是当次报告，不是交易承诺。

本轮没有可靠的每资产绝对解锁时间，页面明确未知，未生成 estimated_unlock 或 actual_unlock；不能靠本地计时转为可出售。后续样本适配、资产迁移和实际解锁观测再进入账本；不同资产不能仅按同名绑定购买记录。

不把这一份库存自动覆盖手工 PositionSnapshot：后者还承载挂牌、未决、预留现金与日量；需要补齐来源、去重/对账规则后才能合并。PlanRequest/PlanPreview 不变。价格历史已有独立的[研究切片](price-history.md)，真实协议仍未验收；不把库存快照历史称为价格历史。

## 验证与下一步

本地命令：`cargo test --locked --offline`、`cargo fmt --check`、`cargo clippy --locked --offline --all-targets -- -D warnings`、`node --check web/inventory.js`。本轮新增 10 项测试，常规 68 项通过、0 失败、2 项默认忽略；夹具覆盖身份/实例/数量/空区/页预算/跨区失败、重复与漂移、认证门互斥、Cookie 轮换、跨账户、冷却持久化、不可覆盖与重启、同源和脱敏。

待办：真实登录后的只读双区样本与空区语义核验 → 资产解锁/跨区迁移观察 → 已挂牌与未决来源及持仓合并 → 价格历史与风险 → 预检/持久化订单。真实平台样本和交易链路仍未验收。

## 2026-10-25 真实账户只读验收

- Steam 独立浏览器登录已通过在线身份核验，凭据经正常应用流程保存。
- 使用已保存会话调用现有库存 API，成功保存一次双区快照：区 2 为 6 条资产，区 16 为 0 条；两区各读一页，均 source_exhausted。
- 本次验证覆盖一个真实账户的非空区与空区读取，不覆盖真实多页、交易冷却迁移、挂牌、未决收货或自动交易。
- 历史快照中的 platform_verified=false 和“协议尚未实测”警告是实现阶段的静态标记；本次实测证据记录于此，不篡改不可覆盖的历史快照，也不据一次样本把所有协议分支标为已验证。
- 未读取或输出 Cookie，未创建订单，未执行买卖。
