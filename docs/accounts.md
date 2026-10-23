# Parallax 账户与手动会话

状态：已实现手动会话和独立浏览器登录/人工恢复切片，详见 [浏览器登录合同](browser-login.md)。Steam Guard/密码自动登录、后台自动续期与交易使用会话均待接入；真实平台登录尚未验收。

## 边界

- `domain/accounts.rs`：账户、平台、验证状态、脱敏视图与 Cookie 输入校验。
- `application/accounts.rs`：创建、导入、验证、停用及并发串行化；不依赖具体存储或 HTTP 框架。
- 应用层端口：AccountRepository、CredentialStore、SessionVerifier。
- `adapters/sqlite/accounts.rs`：仅保存账户/会话元数据、凭据引用与 BUFF 全局冷却/验证阻断。新增表，不改旧扫描/计划快照。
- `adapters/credentials.rs`：Windows Credential Manager 原生后端；使用 `keyring 3.6.3` 的 windows-native，仅当前用户可用。其他平台明确返回不支持；没有明文回退。
- `adapters/session.rs`：只读平台身份验证，证书验证开启、15 秒请求超时、限制响应体、手动控制同主机 HTTPS 重定向，不输出 Cookie、响应体或带凭据错误。
- `transport/accounts.rs`：独立 API 路由与请求体，无 Cookie 导出/回显，响应 no-store；复用同源写入检查。
- `web/accounts.js`：表单、状态显示和统一 API 调用；成功保存后清空 Cookie，失败保留输入；不使用 localStorage。

## 使用

运行 `./dev.ps1 run` 后，在页面底部创建账户（备注与 17 位 Steam ID）。选择 Steam 或 BUFF，粘贴 **Cookie 请求头**，可附浏览器 User-Agent。保存只会得到 unverified；点击在线验证才会发出只读请求。

Steam 通过 `/my/?xml=1` 解析服务器返回的 steamID64，与账户比对；跳转登录页视为失效，未知页面或网络异常视为 unknown。BUFF 使用旧版同一 `account/api/user/info/v2` 协议，要求返回用户 ID 与匹配的绑定 Steam ID；缺少可识别身份返回 unknown。这些非公开页面/协议可能变化，尚待用户新导入凭据进行实测。

## API

| 方法与路径 | 请求 | 响应 |
| --- | --- | --- |
| GET /api/accounts | 无 | `{items: Account[]}` |
| POST /api/accounts | `{label, steam_id}` | 201 Account |
| GET /api/accounts/{id}/sessions | 无 | `{items: SessionView[]}` |
| POST /api/accounts/{id}/sessions/{platform} | `{cookies, user_agent?}` | SessionView；只保存不联网 |
| POST /api/accounts/{id}/sessions/{platform}/verify | `{}` | SessionView；一次只读验证 |
| DELETE /api/accounts/{id}/sessions/{platform} | 无 | SessionView；只停用本机凭据 |

platform 为 steam/buff；类型见 `web/lib/api.d.ts`。SessionView 含 platform、generation、status、checked_at、retry_at、message、subject。generation 是不透明版本标识，不是 Cookie。账户显式通过路径传入，无共享的全局当前账户。

状态为 unverified、valid、invalid、unknown、rate_limited、verification_required、missing。valid 表示上次只读身份检查通过，不能当成永久有效，也不能据此创建订单。

## 一致性与限制

- 新凭据先写入随机版本的系统条目，再提交 SQLite 引用，提交成功后清理旧条目。元数据提交失败会尝试清理新条目；清理异常显式报错。进程恰在写 vault 后崩溃仍可能留下未引用条目，后续需要恢复清理机制。
- 同一应用实例串行处理凭据替换/验证/停用。每实例使用独立数据库，尚无跨进程认证事务锁。SQLite 和系统凭据存储无法形成单一事务。
- Windows 单条凭据限制 2560 字节，使用二进制 JSON 存储；输入 Cookie 与 UA 合计上限 2300 字节，JSON 超限仍明确拒绝。长浏览器 Cookie 需仅保留请求头所需字段。
- 每次验证前先持久化 30 秒节流；BUFF 额外全局节流。429 以 Retry-After 秒数且至少 300 秒处理（HTTP 日期暂按 300 秒），最多 7 天；替换凭据或新建本地账户不能绕过冷却。
- BUFF 验证挑战保存为全局阻断，Cookie 导入、停用、重启均不能解除。独立浏览器捕获、只读身份核验和新会话保存全部成功后才显式解除；恢复仍尊重全局冷却。
- 停用先写 missing 并更新凭据代次，再删除凭据；旧浏览器窗口不能恢复后来停用的登录态。不代表平台远程注销。删除失败会返回错误，本地仍保持不可用。没有删除账户/旧库迁移/复制原配置能力。
- BUFF 会话已用于显式只读卖单请求，复用同一认证串行门、冷却和挑战政策，见 [现金端合同](buff-quotes.md)。没有将会话连接到 SteamDT 扫描、付款、真实买卖、礼物结账或自动后台请求。

## 复用选型

采用 [keyring 3.6.3 文档](https://docs.rs/crate/keyring/3.6.3/source/README.md) 描述的 Windows 原生存储（MIT/Apache-2.0）。该版本支持现有工具链和 native feature；未采用手写 Windows FFI。浏览器适配复用 headless_chrome，选型与限制见 [浏览器登录合同](browser-login.md)，捕获后仍使用同一身份验证器。

## 验证

`cargo test --locked --offline` 的会话隔离、冷却、重启阻断、网络未知、凭据缺失、脱敏错误和同源检查均使用合成数据，不访问真实平台。

Windows 后端独立检查：`cargo test --locked --offline native_vault_roundtrip -- --ignored`。该项仅创建/读取/删除一个随机命名的合成凭据，不读取用户已有条目。夹具与原生后端验证都不能证明真实 Steam/BUFF 响应已适配。
