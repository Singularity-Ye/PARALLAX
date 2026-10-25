# 策略导入导出：M09细则继承

2026-10-25。原项目reference `app/strategy_engine.py:1198–1215`的import_strategy先规范化/验证再保存，export_strategy输出副本并去掉创建/修改时间；路由位于app/routes/strategies.py。继承其规则备份、迁移和复用目的，不机械复制全局配置或旧策略ID覆盖行为。

## 当前实现

策略中心“策略备份与导入”可导出选中的已保存版本，查看JSON并下载；不会包含未保存编辑。支持读取JSON文件或粘贴，点击导入后创建新策略v1，旧策略/历史版本/当前编辑保留，不激活执行。导入前需核对规则，不自动请求平台或生成研究。

包结构：`{"format":"parallax_strategy","format_version":1,"draft":{...原StrategyDraft...}}`。domain/strategy_transfer定义显式格式，复用StrategyDraft的schema1/2/3、顺序/依赖、参数与模块校验；application/strategy复用create/version及现有版本仓储，无新表。包不含账户、凭据、研究、策略ID、时间或激活状态；自由说明文字是用户内容，分享前仍应自行核对。

GET `/api/strategies/{id}/versions/{revision}/export`：200返回该准确版本包；404不存在。POST `/api/strategies/import`：请求为包，201返回新StrategyVersion；格式版本/业务参数400，结构/未知字段/未知模块422，跨站403，超过16KiB为413。沿用同源和no-store。

前端lib/strategy-transfer.js只负责文件/JSON/反馈，API集中api.js与api.d.ts；原表单与规则引擎不复制。保存后列表刷新失败不谎称导入失败。每次成功POST是显式创建新副本，不保证重复请求去重；网络结果未知时先刷新列表核对，当前不自动重试。

## 尚未迁移

原项目Python/旧JSON结构不直接兼容；自定义声明式模块、激活、出售策略完整配置、流程步骤调度和模拟仍待。不能把本切片标为M09全部完成。仅将旧配置改名为Parallax包不会绕过规则校验，也不允许导入代码或执行Python。

## 运行

`./dev.ps1 run`，本机38480端口；定向`cargo test --locked --offline strategy_transfer`。测试用合成策略和内存数据库，不连接平台。

## 验证结果

新增HTTP集成测试通过：指定旧版本往返、新ID/v1、原两版本保留、不支持格式/未知字段/未知模块/空规则拒绝、跨站403、16KiB限制、no-store。fmt/Clippy/JS语法/debug构建通过。本切片未重跑全量测试。

真实页面导出已有“独立持有历史观察初版”v1成功，下载入口可见；非法JSON反馈留在备份区域且保留输入，重新导出可继续，无脚本错误。没有向真实数据库导入测试副本，也未点击文件下载；文件导入/成功保存往返由隔离HTTP测试覆盖。服务更新使用原数据库和环回38480端口。
