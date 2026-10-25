# Parallax BUFF 订单观察与精确查找

2026-10-25：已实现，只读协议适配和合成夹具通过；尚未完成真实账户协议验收。它是后续对账的数据入口，不是完整订单执行器、支付核验或资金来源。

## 原项目思路与有意差异

复用两版本 `buff/buyer.py` 的 `API_HISTORY=/api/market/buy_order/history`。reference 的 `check_wait_pay_orders` 使用 `PAYING` 与 `pay_expire_timeout > -1`；audit 的 `is_order_paid/get_order_detail` 仅查最近20条，并将3/4/5视作已支付。重制保留准确订单号和只读观察，补分页/范围/失败/未知的区别；**未继承未经本项目实测的数字状态付款映射**。

现有源码没有明确的 BUFF 真实可用现金/冻结余额合同，本轮不猜余额 URL 或金额字段，也不把订单数换算成余额。人工 CNY 资金观察保持 manual_unverified，Steam UAH 等钱包独立。

## 复用与分层

- `domain/buff_orders`：观察请求、分页证据、状态码、停止原因及准确ID查找。无网络/存储依赖。
- `application/buff_orders`：持有同一个 Accounts 认证门，读取后一次保存；仓储与 provider 可替换。
- `application/accounts_buff`：从既有卖单流程提取共享租约，复用有效身份/五分钟时效、凭据后端、代次与全局挑战/冷却。应用内串行至保存完成；不是完整跨进程平台活动锁。
- `adapters/buff_read`：复用原 reqwest GET 客户端配置、20秒超时、禁止重定向、2MiB上限和 Cookie 变化/挑战/429 分类。卖单和订单历史共用，没有引入新框架或依赖。请求地址由适配器固定，不接受用户 URL。
- SQLite 新表 `buff_order_observations` 只 INSERT；保存事务重查当前 BUFF 身份/有效状态/代次，读取期间改变则返回冲突。分页前使用 `try_buff_read` 原子领取两秒节流，不得清除期间持久化的挑战/冷却。
- `web/buff-orders.js` 在“操作记录”接线，复用纸感主题、卡片、forms 和统一 API/types。后端解释状态，页面只展示/选择。

## 范围与诚实状态

请求 `max_pages=1..5`、`page_size=1..80`，只读 CS2/csgo 历史，连续页面相隔两秒。保存起止时间、账户/BUFF主体/凭据代次、每页条数/上游总数与以下终止原因：

| stop_reason | 含义 |
| --- | --- |
| short_page | 观察到不足一页的结果 |
| reported_end | 观察条数达到上游报告的总数 |
| page_budget | 到达用户设定的页数上限 |

即使达到上游末尾也固定 `complete=false`，因为跨页不是平台同一时刻的原子清单。缺失不等于订单不存在、已取消或支付失败；中途失败不保存部分成功。页号错误、同页/跨页重复、总数漂移、短页与总数冲突拒绝新快照，旧快照不变。没有总数时保留未知，不以缺省0冒充空账户。

只投影订单 ID、原始字符串/整数状态码、可选整数秒支付窗口；若响应有 buyer_id 则核对 BUFF 主体，不保存用户资料、支付链接、Cookie或任意原始JSON。缺失状态/窗口保留null；畸形字段拒绝。协议未确认的格式不做宽松猜测。

只有保存时 `PAYING` 且窗口非负输出 `awaiting_payment_at_observation`，它不是当前状态。数字3/4/5、其它字符串、缺字段或负窗口均输出 `unknown`。没有 `paid=true`、平台取消或支付重试动作；不会自动改台账或释放本地预留。

## HTTP 与运行

账户前缀 `/api/accounts/{account}`，同源、no-store、2KiB请求体：

| 方法 | 路径 | 行为 |
| --- | --- | --- |
| GET / POST | `/buff-order-observations` | 最近10份不可覆盖快照 / 只读获取并保存，201 |
| GET | `/buff-order-observations/{id}` | 当时完整证据，不刷新平台 |
| POST | `/buff-order-observations/{id}/lookup` | `{order_id}` 精确查保存页面，200 |

观察POST示例：`{"max_pages":2,"page_size":20}`。lookup找到为found=true，范围内未找到仍200/found=false/payment_evidence=unknown；其它账户或不存在快照404。参数400、未知字段422、跨站403、身份/验证冲突409、冷却429（retry_at）、平台/协议失败502。回看和查询不需要仍然有效的凭据，不访问凭据后端/平台。

```powershell
./dev.ps1 run
cargo test --locked --offline buff_order
# 独立验收实例必须使用不同数据库与端口
$env:PARALLAX_DB = '.run/orders-qa.db'
$env:PARALLAX_PORT = '38483'
```

## 验证与待办

新增13项（解析2、领域3、用例/HTTP8），完整 **182通过、0失败、2默认忽略**。覆盖状态未知、分页/缺失、故障保留、挑战/冷却、Cookie代次变化、保存前CAS、分页期间新增阻断、跨账户/同源/未知字段及秘密字段投影。原卖单/账户回归沿用同一公共层。

隔离合成库页面验证保存记录恢复、PAYING的当时语义、数字3仍付款未知、缺失仍未知、凭据不满足时阻断读取并保留输入/结果，以及切页保留。两条订单均合成；未使用真实凭据或请求平台。截图只留工程 `.run/orders-qa.png`，验收实例收口关闭。

- [ ] 真实 BUFF 账户的历史分页/总数字段、订单身份和状态枚举脱敏样本验收。
- [ ] 真实可用资金/冻结款接口的可靠合同与地区/币种核验。
- [ ] 精确外部订单引用和本地发送尝试关联、支付金额/币种/主体核对。
- [ ] 未知发送结果对账、完整持仓/费用/实时复核及单次执行授权。

相关：[本地采购准备](checkout.md)、[迁移地图](migration-map.md)、[进度口径](progress.md)。
