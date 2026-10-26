# 代理节点与固定平台路由

## 2026-10-26 重启后补验

新增两项定向测试通过：本机合成代理收到HTTPS CONNECT目标，隧道握手不包含目标请求的合成Cookie或路径，代理返回502后请求失败；HTTP接口检查no-store、跨站拒绝、未知节点拒绝及未接浏览器/自动轮换的能力标记。CONNECT测试没有建立成功TLS隧道，不能替代真实HTTPS代理与证书链验收。Clippy通过，未重跑全量测试，未访问真实平台。

本轮只修改后端测试和本文；前端动效由反重力在共享工作区负责，未纳入本轮提交。后续合入需核对其基线、文件清单和验证结果；统一复用theme/components的视觉与动效定义，不在业务脚本复制动画实现。正式服务已使用release程序在38480启动并通过首页/代理状态只读检查；本轮测试变更不需要重启服务。

2026-10-26阶段收尾。已实现进程启动配置、节点池、平台固定出口、只读状态与手动匿名连通性测试。尚无页面配置保存、热切换、自动轮换/故障转移、SOCKS、账号密码代理或Webshare导入；不宣称完整原代理池迁移完毕。

原reference app/routes/proxy.py提供保存/重载、并发测试、Webshare采集与清空，utils/proxy_manager.py为路由方向。当前先继承配置/路由与测试目的，复用现有reqwest代理能力，不自写HTTP隧道。原随机轮换不直接用于认证会话。参考：https://docs.rs/reqwest/latest/reqwest/struct.Proxy.html 。

配置示例（另存被忽略的data/proxy.json，仅示例地址，不自动创建/启用）：

```json
{"enabled":true,"nodes":[{"id":"local","url":"http://127.0.0.1:7890"},{"id":"backup","url":"http://127.0.0.1:7891"}],"routes":{"steam":"local","steamdt":"local","buff":"direct","fx":"direct"}}
```

启动前设置PARALLAX_PROXY_CONFIG为文件绝对路径，再用./dev.ps1 run。默认不设置时保持原系统环境代理行为，不改变当前用户网络。enabled=true时只有配置路由走指定节点，未配置和direct直连；节点ID唯一、最多20个，路由引用须存在；仅支持HTTP/HTTPS且禁止URL账号密码、路径/查询/片段。配置错误启动失败，不静默忽略。不写入或读回其它软件的代理凭据。

adapters/network统一构造reqwest客户端，7个适配器共用：行情、汇率、身份验证、BUFF读取、Steam历史/库存/盘口，TLS provider也收拢。请求域名映射steam/buff/steamdt/fx，同平台固定出口，不轮换、不在失败后改走直连。代理失败由原访问门处理，不能靠换出口解除挑战/限流。浏览器/CDP登录窗口不在此代理配置范围内，需后续独立接线与真实环境验收。

GET /api/network/proxies返回模式、节点ID与路由（不回显URL），browser_routed=false、automatic_rotation=false、requires_restart=true。POST /api/network/proxies/{id}/probe仅经该节点匿名GET https://steamcommunity.com/，8秒超时、禁止重定向、一次一项并发，返回HTTP响应/连接失败与耗时；没有Cookie，不代表身份验证或实际IP已确认。no-store/同源。UI为进程当前配置展示/测试，未部署及浏览器验收。

测试用本机合成HTTP代理核对绝对URL请求与失败不直连回退，DNS强制loopback，不访问真实Steam；sandbox套接字拒绝时在主机运行同一测试通过。后续仍需真实节点、HTTPS CONNECT、证书链、跨分页固定出口和系统代理分支验收。
