# Steam Guard令牌切片

2026-10-26阶段收尾。已实现账户绑定密钥导入/本机停用、显式生成5位登录验证码与30秒有效期；没有实现密码自动登录、手机确认登录、会话续期或交易确认。生成验证码不证明账号身份或登录成功。

原reference app/routes/auth.py的_generate_steam_guard_code和/api/steam_guard已核对，使用shared_secret、HMAC-SHA1、30秒计数与Steam字符表。原services/steam_auth.py的steampy密码/挑战/续期是另一条链，未以本切片冒充完成。没有读取旧配置、用户maFile、Watt Toolkit或其它软件的真实密钥。

领域steam_guard使用既有ring和base64依赖：密码运算交成熟ring，只有Steam编码/时间窗口是协议适配。比较过steam_guard（旧sha1/base64依赖）与steamguard（含完整登录/确认能力），本切片复用已锁定依赖避免引入第二套网络/账号模型；完整登录库选型后续再做。来源：https://docs.rs/steam_guard/latest/steam_guard/ 、https://docs.rs/steamguard/latest/steamguard/ 。

application/steam_guard通过AccountRepository和GuardSecretStore绑定account_id+steam_id并串行处理，NativeVault沿用Windows凭据后端，独立服务名Parallax.steam-guard.v1。密钥不写SQLite、不返回状态响应、不derive Debug/Serialize；正常导入替换该账户本机密钥。本地停用不解绑Steam手机令牌、不注销Cookie。当前Windows后端真实密钥写入未实测，测试用内存vault。

GET/POST/DELETE /api/accounts/{id}/steam-guard；POST请求{shared_secret}，标准Base64解码需20字节，512B请求限制；GET/POST只返回配置状态。POST同路径/code才返回code/generated_at/valid_until/period_secs/time_basis=local_system_clock/scope=login_code_only/automatic_login_available=false。同源、no-store，账户/密钥缺失404、格式400、结构422。使用本机时钟，未请求Steam校时；导入也不能验证密钥确属所选账号。

页面复用账户/API和纸感表单，有有效期进度与到期清除，不后台连续取码；切换页面/隐藏窗口清除码，离页清除密钥输入。尚未浏览器验收/部署到当前服务。后续先在隔离数据库和合成密钥验证页面，不往真实库写测试密钥。

测试包括公开合成20字节密钥的独立Python HMAC参考向量、29/30秒边界、格式拒绝、HTTP同源/no-store、账户隔离与内存停用；不包含真实Steam登录。

阶段门：主机侧完整217通过、0失败、2默认忽略；Clippy/fmt/JS检查通过。尚无生产二进制构建、页面与真实vault验收，按用户要求暂停。
