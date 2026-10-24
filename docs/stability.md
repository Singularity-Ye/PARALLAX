# 自然日波动与趋势描述 v1

2026-10-24。M08 的 IQR/CV/EMA/趋势/R²基础指标已接入价格历史研究。当前不产生“可以买/稳定通过”的结论，不修改扫描、预算或原七自然日P10。完整策略阈值、布林带及买卖条件仍待迁移。

## 来源与复用

提取原 `analysis/stability.py` 的样本CV、IQR、EMA/回归和 `utils/trend.py` 的相对斜率思路，复用 `risk::daily_bars_scaled` 的原币日量加权样本价。不建立第二套行情、日聚合、账户或FX。样本是已量化日价，不能把日点数/接口记录数当真实交易笔数。

检查了现有依赖及 [statrs Statistics](https://docs.rs/statrs/latest/statrs/statistics/trait.Statistics.html)（MIT、通用浮点统计）。本切片需要整数金额、有理数IQR、自然日缺口和可空结果，通用样本统计不能直接覆盖合同；不新增分布/矩阵依赖，仅在领域层实现有界规则。样本CV定义参照 [NIST](https://itl.nist.gov/div898/software/dataplot/refman2/auxillar/coefvari.htm)。主要成本为至多365个日价排序，网络与存储成本不变。

## 计算口径

| 输出 | 方法与未知条件 |
| --- | --- |
| usable_days / missing_or_unusable_days | UTC窗口内正量且可量化为Money的日价；无记录/零量/缺量/低于最小单位不补价 |
| raw_sample_cv_bps | 全部有效日价的样本标准差（n−1）/均值；n<2为null，平价为0。整数求和/平方和，只有无量纲平方根用f64，最终bps向上取整 |
| IQR | 线性插值Q1/Q3、1.5×max(IQR,5%均值)；n<3不清洗。以有理数精确比较边界，显示下界floor/上界ceil，不以取整边界过滤 |
| clean_sample_cv_bps / mean / median | 仅IQR保留日价的描述；Money向下量化，少样本CV为null，列出每个排除日期 |
| EMA7 / EMA30 | 固定最近7/30个已结束UTC自然日，必须每天有有效价，否则null并返回usable_days。alpha=2/(span+1)，窗口首日作seed；递推保留1e6子最小单位，输出Money向下量化 |
| trend7.relative_slope_bps_per_day | 最近七自然日有效价OLS，x为真实日期偏移，允许缺日但报告点数；斜率除窗口均价，signed bps/自然日向下取整；n<2为null |
| trend7.r_squared_bps | 含截距OLS平方相关系数，整数交叉乘法取整到0–10000；常量序列分母为0时null。不是置信度或概率，n<5另提示少样本 |
| reference_range_position_bps | 最后有效历史价在清洗后min/max间的位置，可负或>10000；无数据/min=max为null。不是经验CDF或当前市场价分位 |

原版0.5元IQR固定底和CNY价格分段不能直接套给外币。本版仅保留5%相对均值buffer，对单位缩放不敏感，与旧固定底有意不同。CV/IQR基于量化后日价，不等同旧版逐历史记录统计。EMA不把7条稀疏记录称为7天，回归不压缩日期。算法 `calendar_stability_v1`。

**IQR只影响清洗后CV/均价/中位数/区间位置；不删除原日线、EMA、趋势或七自然日P10里的极端价格。** 原始/清洗后结果分别展示，避免清掉尾部后看起来稳定。

## 接口与保存

没有新增API或请求参数。原POST `/api/accounts/{account}/price-histories` 的新快照在 `analysis.stability` 保存结果，类型见 `web/lib/api.d.ts`；算法、范围、原币与原日线在同一不可覆盖快照。账户/候选、认证门、冷却和平台调用次数不变。

已知币种但无可用日价时给指标对象与明确null；未知币种不生成指标。旧JSON缺stability仍可读，历史查询不重算；页面明确未包含指标。原analysis.status只描述七日配对，不代表稳定性通过或执行许可。

运行 `./dev.ps1 run`，进入 `http://127.0.0.1:38480/#price-history`。真实SteamDT候选与最近核验的Steam会话可显式读取；演示候选仍禁止请求平台。没有新依赖、数据库迁移或后台网络任务。

## 验证与待办

新增7项测试，常规 **102通过、0失败、2默认忽略**；fmt/Clippy/JS通过。覆盖常量/空/单点/无效日、EMA完整/缺日/手算冲击、稀疏日期回归、负斜率、单位缩放、IQR边界/原始尾部保留、乱序/重复/币种/范围、旧JSON与P10不变。现有HTTP/持久化测试扩展检查序列化、账户隔离和重开结果一致，库存/FX/定价完整回归通过。

真实历史协议及策略收益未验收。本轮未登录或抓取真实历史，未在用户DB生成模拟研究。后续建立版本化策略、参数/样本门槛及显式指标引用，再推进预检/持久化订单；本模块不是完整自动交易策略引擎。
