# UAH 原币与多币种支持

2026-10-24。已将乌克兰格里夫纳 UAH 纳入默认支持集：CNY/USD/EUR/GBP/HKD/TWD/UAH。默认显示/预算仍为 CNY；账户钱包、平台原币和显示币种各自保留，不因显示切换而改变地区或结算。

## 单位与来源

Steam 官方将 UAH 金额报告为 kopiykas，即格里夫纳的百分之一，见 [Steamworks 支持币种与单位](https://partner.steamgames.com/doc/store/pricing/currencies)。社区盘口 `eCurrency=18` 对应 UAH，映射同时核对原项目 `steam/market_orders.py` 与 [steampy 的 Currency 定义](https://github.com/bukson/steampy/blob/master/steampy/models.py)。官方商店币种单位说明不等于社区未公开协议的实测证明。

继续复用 [Frankfurter 官方日参考汇率 API](https://frankfurter.dev/)；UAH 元数据与明确日期的 CNY→UAH 报价已通过匿名只读请求验证。没有新增数据源、固定汇率、依赖或表。比例保持 `units_per_cny_1e8` 整数，不通过浮点倒数换币种。

## 接入路径

| 层 | UAH 行为 |
| --- | --- |
| domain/fx | 支持 UAH 的整数交叉换算；旧六币种快照仍能校验/读取，缺 UAH 的旧快照不能生成 UAH 金额 |
| application/fx | 当前共享缓存须覆盖七币种；旧缓存缺 UAH 时刷新并保存新快照，不覆盖旧证据。刷新失败保留旧记录，返回失败/冷却，不伪造 UAH 比率 |
| adapters/fx | 请求币种从共享支持集生成，响应须完整包含六个外币行（加本地 CNY 锚点共七币种）；日期/范围/TLS/大小限制不变 |
| Steam 盘口 | 可显式请求 UAH，映射至 Steam 18；解析实际 eCurrency=18 为 UAH。请求币种和实际返回分别保存，失配给出说明 |
| Steam 历史 | 识别明确 UAH、₴、грн、грн. 前/后缀；冲突标记保持未知。原始价格保留1e4精度，先做原币自然日聚合，再量化为 kopiykas |
| 定价/费用/预算/策略 | 沿用已有原币壁垒定价、FeeSchedule、FX和证据模型；原币费用先扣再换显示。预算继续CNY模型估算，UAH原价与完整汇率进入历史计划/研究，不冒充已核验账户费用 |

`OrderBookRequest.currency` 是新增可选字段，省略默认 CNY；CNY 序列化继续省略该字段，保持旧 JSON 往返。`requested_currency` 保存实际发出的请求值，`book.currency` 始终以平台响应为准，二者不同不能改标签修补。旧快照不会重抓或重算。

## 最小使用

运行 `./dev.ps1 run`，打开 `http://127.0.0.1:38480/#steam-research`。显示币种及请求币种都可选择 UAH；请求币种是提示，平台可能忽略。换算已保存的盘口不刷新盘口观察时间。

```json
{"app_id":730,"market_hash_name":"Prisma 2 Case","currency":"UAH"}
```

POST /api/steam/order-books。返回同时含 `request.currency`、`requested_currency` 和 `book.currency`。POST /api/steam/order-books/{id}/valuation：

```json
{"display_currency":"UAH"}
```

定价研究同样可选 UAH 显示；如果实际盘口为 UAH，费用参数必须按 UAH 最小单位填写，不能填人民币最低费后强行换标签。启用费用模型仍是显式研究假设，真实钱包参数尚待账户模块/平台核验。历史读取使用登录账户实际返回的明确币种，不在历史 URL 伪造地区。

## 已验证与待验收

新增9项测试，完整 **126通过、0失败、2默认忽略**；fmt、Clippy、JS语法与debug构建通过。覆盖UAH原币单位、报价请求映射、实际返回失配、历史精度/日线/P10、原币费用/显示/预算/策略贯通、旧六币种JSON、缓存补齐/失败/不完整响应。测试中的UAH盘口和费用是合成数据。

公开接口实测：Frankfurter明确日期2026-10-24返回完整六个外币行，含UAH；本机页面请求UAH的Prisma 2 Case盘口成功，Steam实际返回TWD。页面保存请求UAH/实际TWD，TWD47.00按参考率显示UAH65.18，原盘口时间未刷新。用户库只新增真实公开盘口及真实参考汇率，没有模拟UAH平台数据、登录、订单或账户地区修改。截图在忽略目录.run/uah-currency.png。

待验收：真实乌区账户的原生UAH盘口/历史、钱包费率/最低费/价格步长和结算；其他视图的显示币种及账户地区同步。此前出售估值联动仍待继续，本切片没有实现自动出售或把旧预览改成订单。

## 2026-10-25 真实行情与乌区历史验收

复用现有API完成SteamDT单页真实扫描：10条候选，5条通过、5条拒绝。随后对首个真实候选读取90天Steam价格历史，保存成功；响应明确识别为UAH，响应1351条，窗口内693条样本。此为单个真实商品样本，未证明所有日期格式/币种/商品分支均通过，也不证明钱包费率或结算。

匿名盘口请求UAH时，平台实际返回eCurrency=8，现有适配器拒绝未支持币种，未保存误标UAH的报价。账户历史UAH与匿名盘口返回币种分别记录，未改变账户地区、未猜汇率、未下单。
