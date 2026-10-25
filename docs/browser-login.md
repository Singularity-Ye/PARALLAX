# Parallax 浏览器登录与人工恢复

## 范围

已接交互式 Steam/BUFF 登录：独立临时浏览器 → 用户在官方页面登录/完成挑战 → 捕获对应平台 Cookie → 只读在线身份核验 → 保存系统凭据。密码、短信及 Guard 码直接输入官方页面，Rust 不代填、不保存。

这不是密码/Guard 自动登录、后台会话自动续期或交易执行。Steam 单个真实账户的浏览器登录、在线身份核验、凭据保存及后续库存读取已通过；BUFF、后台续期和完整会话生命周期仍待验收。

## 接口与使用

运行 `./dev.ps1 run`，在账户区创建/选择账户及平台，点击“打开独立登录窗口”。完成官方页面登录后回到 Parallax，点击“完成登录并验证”。失败保留窗口，可以在冷却结束后重试或取消。刷新 Parallax 页面通过 current 接口找回待处理流程。

| 方法与路径 | 行为 |
| --- | --- |
| POST /api/accounts/{account}/sessions/{platform}/browser | 201；绑定账户、平台及启动时凭据版本，打开独立窗口 |
| GET /api/browser-logins | `{item: LoginView|null}`，当前或最近一轮状态 |
| POST /api/browser-logins/{id}/finish | 捕获并验证；成功后保存，重复成功请求返回同一结果 |
| DELETE /api/browser-logins/{id} | 取消并关闭窗口，不替换原登录态 |

LoginView 包括 id、account_id、platform、status、expires_at、message、session；status 为 pending/completed/cancelled/expired。响应 no-store，不含 Cookie、UA、调试端口或 profile 路径。不存在流程返回 404，已结束、版本变化或冷却返回 409，捕获/身份失败返回 400。

同一进程一次只接受一个登录窗口；有效期 600 秒，运行时定时清理，GET/finish/start 也检查过期。重启不恢复临时流程。新流程替代已结束流程后，旧 id 无效。终态重放不会重新捕获或重验；会话的实时有效性以账户 sessions 接口为准。

## 复用与选型

复用 [headless_chrome 1.0.22](https://docs.rs/headless_chrome/1.0.22/headless_chrome/)（MIT）的 CDP、进程管理与临时 profile；依赖锁定，offline feature 在构建时使用内置协议，不启用 fetch 自动下载浏览器。同步 CDP 调用放 spawn_blocking，不阻塞异步执行器。

短选型比较：

| 方案 | 适配判断 |
| --- | --- |
| headless_chrome | 支持可见窗口、Cookie 和独立临时目录，现有切片无需管理异步事件流，采用 |
| [chromiumoxide](https://github.com/mattsse/chromiumoxide) | MIT/Apache-2.0，异步 CDP 适合后续大范围自动化；本切片只需少量调用，暂不增加 handler 管理 |
| [fantoccini](https://github.com/jonhoo/fantoccini) | MIT/Apache-2.0，成熟 WebDriver 客户端；还需配置匹配的驱动服务，当前优先复用本机 Edge/Chrome |

三者均有可查上游仓库与发布/文档；这是集成成本选择，未做性能排名。若后续升级，先重跑隔离浏览器与协议夹具。

## 平台、凭据与并发边界

- 默认发现本机 Chrome/Edge，或通过服务器环境变量 PARALLAX_BROWSER 指定可执行文件。浏览器必须由本应用新启动，不连接用户原有浏览器，不读取旧项目/个人 profile。
- 本地调试地址仅环回，随机端口；浏览器沙箱、TLS 证书验证开启，不使用反检测或绕过验证手段。用户在可见官方页面自行登录；完成时页面须返回 steamcommunity.com 或 buff.163.com。
- 仅取对应 HTTPS 页面适用的 Cookie，再按域过滤。Steam 保存 steamLoginSecure/sessionid，BUFF 保存 session/csrf_token；同名值冲突拒绝，不混合商店与社区 Cookie。当前仅社区会话，不提供 Steam 商店/礼物会话。
- 捕获不等于登录成功：复用 SessionVerifier 匹配 Steam 身份；unknown、限流、失效、挑战均不覆盖原凭据。
- import/revoke 和浏览器提交复用 Accounts 串行门；开始时绑定凭据代次。后来导入或停用会改变代次，旧窗口不能覆盖新决定。
- BUFF 恢复仍尊重全局冷却；挑战阻断只允许在浏览器捕获、在线核验和新会话保存均成功后解除。普通验证、手工替换与重启不能解除。
- 失败验证更新已有会话的 retry_at；BUFF 节流和挑战持久化。首次 Steam 登录尚无旧会话时，额外冷却仅在当前流程内，取消/重启后不保证保留；不声称已实现平台级全局节流。
- 系统凭据仍有 Cookie+UA 2300 字节、原生条目 2560 字节上限。临时 profile 由库尽力清理；进程被强杀或系统崩溃可能留下临时目录，不承诺安全擦除。SQLite 与凭据库仍不是同一事务。

## 验收与下一步

常规本地测试使用合成 BrowserAuth/SessionVerifier，覆盖账户/版本绑定、重复完成、取消/过期、错误身份、冷却、旧凭据保护和 BUFF 挑战恢复。`cargo test --locked --offline isolated_browser_smoke -- --ignored` 另行启动独立无界面浏览器，仅访问 about:blank，检查空 Cookie 与非平台页面拒绝，不访问真实账户。

下一步：用户操作下的真实平台登录验收；验证过期/挑战后的人工恢复；随后接真实报价、历史、库存。密码/Guard 自动登录及未决支付时冻结凭据替换需独立切片，真实交易仍禁用。

## 登录XML路由循环修正

2026-10-25用户实机反馈：/my/?xml=1触发登录循环，但同窗口/my/正常；旧程序误将第一次登录跳转当成会话失效。OnlineVerifier改为/my/认证重定向/准确SteamID核对，保留同源HTTPS与账户匹配；公开XML不单独授权。184项通过/2忽略，随后真实完成保存成功，并使用保存会话读取库存成功。没有改Cookie捕获/Windows凭据存储或自动获取密码。重启会结束旧临时登录流程，账户记录保留。
