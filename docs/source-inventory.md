# Parallax 原项目源码盘点

生成方式：`python tools/source_inventory.py --output docs/source-inventory.md`。仅静态解析，不导入或运行旧项目。

覆盖 Git 跟踪文件和未忽略的新文件；配置、凭据、数据库与图片只记路径，不读取内容。符号盘点完成不等于行为迁移完成；以下行号相对本地工作树，SHA-256 锁定源码内容。

## AetherSwap_reference

HEAD：`02552e5d9c7b2fe237ea908017819c9f0a09cb69`；文件 180；Python 120。

本地差异（不会覆盖）：
```text
M app/services/iflow_client.py
 M app/services/steam_auth.py
 M steam/market_orders.py
 M steamdt/fetcher.py
 M tests/test_market_orders.py
 M tests/test_steam_auth_errors.py
 M web/js/accounts.js
?? .agreed_disclaimer
?? config/server-28473-autotest-20261012-110305.stderr.log
?? config/server-28473-autotest-20261012-110305.stdout.log
?? config/server-28473-current-20261011-221346.stderr.log
?? config/server-28473-current-20261011-221346.stdout.log
?? config/server-28473-host-20261011-221658.stderr.log
?? config/server-28473-host-20261011-221658.stdout.log
?? config/server-28473-restart.stderr.log
?? config/server-28473-restart.stdout.log
?? config/server-28473.stderr.log
?? config/server-28473.stdout.log
?? tests/test_steamdt_proxy.py
```

| 路径 | 分组 |
| --- | --- |
| `.agreed_disclaimer` | 启动、包入口与工程资产 |
| `.dockerignore` | 启动、包入口与工程资产 |
| `.env.example` | 启动、包入口与工程资产 |
| `.gitignore` | 启动、包入口与工程资产 |
| `Dockerfile` | 启动、包入口与工程资产 |
| `LICENSE` | 启动、包入口与工程资产 |
| `README.md` | 启动、包入口与工程资产 |
| `analysis/__init__.py` | 策略与估值 |
| `analysis/stability.py` | 策略与估值 |
| `app/__init__.py` | 启动、包入口与工程资产 |
| `app/__main__.py` | 启动、包入口与工程资产 |
| `app/accounts.py` | 账户与认证 |
| `app/api.py` | 启动、包入口与工程资产 |
| `app/config_loader.py` | 持久化与配置 |
| `app/config_schema.py` | 持久化与配置 |
| `app/database.py` | 持久化与配置 |
| `app/gift_engine.py` | 游戏折扣与礼物 |
| `app/inventory_cs2.py` | Steam 行情与库存协议 |
| `app/main.py` | 启动、包入口与工程资产 |
| `app/notify.py` | 启动、包入口与工程资产 |
| `app/pipeline.py` | 交易生命周期 |
| `app/pipeline_context.py` | 交易生命周期 |
| `app/pipeline_steps.py` | 交易生命周期 |
| `app/purchase_limit.py` | 策略与估值 |
| `app/receive_flow.py` | 交易生命周期 |
| `app/repair_error_records.py` | 交易生命周期 |
| `app/routes/__init__.py` | HTTP 入口 |
| `app/routes/accounts.py` | HTTP 入口 |
| `app/routes/auth.py` | 账户与认证 |
| `app/routes/config.py` | HTTP 入口 |
| `app/routes/gift.py` | 游戏折扣与礼物 |
| `app/routes/inventory.py` | HTTP 入口 |
| `app/routes/pipeline.py` | HTTP 入口 |
| `app/routes/proxy.py` | HTTP 入口 |
| `app/routes/static.py` | HTTP 入口 |
| `app/routes/status.py` | HTTP 入口 |
| `app/routes/steam_deals.py` | 游戏折扣与礼物 |
| `app/routes/strategies.py` | HTTP 入口 |
| `app/routes/transactions.py` | HTTP 入口 |
| `app/runtime_env.py` | 启动、包入口与工程资产 |
| `app/sell_pipeline.py` | 交易生命周期 |
| `app/services/__init__.py` | 基础设施与后台任务 |
| `app/services/account_region.py` | 基础设施与后台任务 |
| `app/services/analysis_client.py` | 基础设施与后台任务 |
| `app/services/buff_auth.py` | 账户与认证 |
| `app/services/buff_checkout_guard.py` | BUFF 协议与执行保护 |
| `app/services/buff_client.py` | BUFF 协议与执行保护 |
| `app/services/deal_cards.py` | 游戏折扣与礼物 |
| `app/services/iflow_client.py` | 行情来源 |
| `app/services/retry.py` | 基础设施与后台任务 |
| `app/services/steam_auth.py` | 账户与认证 |
| `app/services/steam_client.py` | 基础设施与后台任务 |
| `app/services/steam_deals.py` | 游戏折扣与礼物 |
| `app/services/task_queue.py` | 基础设施与后台任务 |
| `app/services/workers.py` | 基础设施与后台任务 |
| `app/shared_market.py` | Steam 行情与库存协议 |
| `app/state.py` | 持久化与配置 |
| `app/steam_confirm.py` | 账户与认证 |
| `app/steam_delist.py` | 交易生命周期 |
| `app/steam_listings.py` | 交易生命周期 |
| `app/strategy_engine.py` | 策略与估值 |
| `app/sync_sold.py` | 交易生命周期 |
| `buff/__init__.py` | BUFF 协议与执行保护 |
| `buff/buyer.py` | BUFF 协议与执行保护 |
| `buff/request_policy.py` | BUFF 协议与执行保护 |
| `config/__init__.py` | 持久化与配置 |
| `config/config.json` | 持久化与配置 |
| `config/server-28473-autotest-20261012-110305.stderr.log` | 持久化与配置 |
| `config/server-28473-autotest-20261012-110305.stdout.log` | 持久化与配置 |
| `config/server-28473-current-20261011-221346.stderr.log` | 持久化与配置 |
| `config/server-28473-current-20261011-221346.stdout.log` | 持久化与配置 |
| `config/server-28473-host-20261011-221658.stderr.log` | 持久化与配置 |
| `config/server-28473-host-20261011-221658.stdout.log` | 持久化与配置 |
| `config/server-28473-restart.stderr.log` | 持久化与配置 |
| `config/server-28473-restart.stdout.log` | 持久化与配置 |
| `config/server-28473.stderr.log` | 持久化与配置 |
| `config/server-28473.stdout.log` | 持久化与配置 |
| `docker-compose.yml` | 启动、包入口与工程资产 |
| `flags/ar.png` | 素材与样例 |
| `flags/az.png` | 素材与样例 |
| `flags/br.png` | 素材与样例 |
| `flags/cl.png` | 素材与样例 |
| `flags/cn.png` | 素材与样例 |
| `flags/hk.png` | 素材与样例 |
| `flags/id.png` | 素材与样例 |
| `flags/in.png` | 素材与样例 |
| `flags/kz.png` | 素材与样例 |
| `flags/ph.png` | 素材与样例 |
| `flags/ru.png` | 素材与样例 |
| `flags/tr.png` | 素材与样例 |
| `flags/ua.png` | 素材与样例 |
| `flags/vn.png` | 素材与样例 |
| `images/1.png` | 素材与样例 |
| `images/2.png` | 素材与样例 |
| `images/3.png` | 素材与样例 |
| `images/4.png` | 素材与样例 |
| `images/5.png` | 素材与样例 |
| `images/steam_deal_card_sample.png` | 素材与样例 |
| `output_cards/1091500_Cyberpunk 2077.png` | 素材与样例 |
| `requirements-server.txt` | 启动、包入口与工程资产 |
| `requirements.txt` | 启动、包入口与工程资产 |
| `run.py` | 启动、包入口与工程资产 |
| `steam/__init__.py` | Steam 行情与库存协议 |
| `steam/client.py` | Steam 行情与库存协议 |
| `steam/inventory.py` | Steam 行情与库存协议 |
| `steam/market.py` | Steam 行情与库存协议 |
| `steam/market_orders.py` | Steam 行情与库存协议 |
| `steam/session.py` | 账户与认证 |
| `steamdt/__init__.py` | 行情来源 |
| `steamdt/fetcher.py` | 行情来源 |
| `steamdt/models.py` | 行情来源 |
| `steamdt/parser.py` | 行情来源 |
| `tests/__init__.py` | 测试与行为合同 |
| `tests/test_account_region_sync.py` | 测试与行为合同 |
| `tests/test_auth_relogin_state.py` | 测试与行为合同 |
| `tests/test_background_startup.py` | 测试与行为合同 |
| `tests/test_buff_ask_seller.py` | 测试与行为合同 |
| `tests/test_buff_auth_hardening.py` | 测试与行为合同 |
| `tests/test_buff_batch_integrity.py` | 测试与行为合同 |
| `tests/test_buff_checkout_guard.py` | 测试与行为合同 |
| `tests/test_buff_checkout_integration.py` | 测试与行为合同 |
| `tests/test_buff_purchase_flow_safety.py` | 测试与行为合同 |
| `tests/test_buff_purchase_persistence.py` | 测试与行为合同 |
| `tests/test_buff_receive_mapping_safety.py` | 测试与行为合同 |
| `tests/test_buff_receive_session.py` | 测试与行为合同 |
| `tests/test_buff_request_policy.py` | 测试与行为合同 |
| `tests/test_buff_verification.py` | 测试与行为合同 |
| `tests/test_config_loader_validation.py` | 测试与行为合同 |
| `tests/test_config_routes.py` | 测试与行为合同 |
| `tests/test_config_schema.py` | 测试与行为合同 |
| `tests/test_config_schema_hardening.py` | 测试与行为合同 |
| `tests/test_currency_conversion.py` | 测试与行为合同 |
| `tests/test_database.py` | 测试与行为合同 |
| `tests/test_holdings_stats.py` | 测试与行为合同 |
| `tests/test_market_orders.py` | 测试与行为合同 |
| `tests/test_pipeline_lock.py` | 测试与行为合同 |
| `tests/test_pipeline_steps.py` | 测试与行为合同 |
| `tests/test_pipeline_time_window.py` | 测试与行为合同 |
| `tests/test_proxy_manager.py` | 测试与行为合同 |
| `tests/test_repair_error_records.py` | 测试与行为合同 |
| `tests/test_runtime_env.py` | 测试与行为合同 |
| `tests/test_sell_listing_response.py` | 测试与行为合同 |
| `tests/test_sell_pipeline_currency_guard.py` | 测试与行为合同 |
| `tests/test_sell_pipeline_ownership_guard.py` | 测试与行为合同 |
| `tests/test_static_routes.py` | 测试与行为合同 |
| `tests/test_status_time_export.py` | 测试与行为合同 |
| `tests/test_steam_auth_errors.py` | 测试与行为合同 |
| `tests/test_steam_client.py` | 测试与行为合同 |
| `tests/test_steamdt_proxy.py` | 测试与行为合同 |
| `tests/test_strategies.py` | 测试与行为合同 |
| `tests/test_strategy_hardening.py` | 测试与行为合同 |
| `tests/test_time_utils.py` | 测试与行为合同 |
| `tests/test_transaction_sale_price_semantics.py` | 测试与行为合同 |
| `tests/test_transactions_delist.py` | 测试与行为合同 |
| `utils/__init__.py` | 基础设施与后台任务 |
| `utils/delay.py` | 基础设施与后台任务 |
| `utils/money.py` | 策略与估值 |
| `utils/network_check.py` | 基础设施与后台任务 |
| `utils/proxy_manager.py` | 基础设施与后台任务 |
| `utils/throttle.py` | 基础设施与后台任务 |
| `utils/time.py` | 基础设施与后台任务 |
| `utils/trend.py` | 策略与估值 |
| `web/css/animations.css` | 前端 |
| `web/css/base.css` | 前端 |
| `web/css/components.css` | 前端 |
| `web/css/layout.css` | 前端 |
| `web/css/variables.css` | 前端 |
| `web/index.html` | 前端 |
| `web/js/accounts.js` | 前端 |
| `web/js/console.js` | 前端 |
| `web/js/gift.js` | 前端 |
| `web/js/main.js` | 前端 |
| `web/js/proxy.js` | 前端 |
| `web/js/settings.js` | 前端 |
| `web/js/steamDeals.js` | 前端 |
| `web/js/steamGuard.js` | 前端 |
| `web/js/strategies.js` | 前端 |
| `web/js/theme.js` | 前端 |
| `web/js/transactions.js` | 前端 |
| `web/js/utils.js` | 前端 |

### analysis/__init__.py

SHA-256：`bae3e2ac4c76bbbf4a96a25434988ee1bb0286b93ee280ec7ff63eb60536b135`

符号：包入口，无定义

依赖：`.stability`

### analysis/stability.py

SHA-256：`c3928025a8a62d85342efb4266bed0e63f442719acbda18b5229404d4465dcbc`

符号：`_percentile:21`、`_iqr_bounds:32`、`clean_prices_iqr:44`、`_vwap_iqr:55`、`_apply_currency:68`、`_parse_item_date:71`、`_safe_volume:73`、`_linear_regression_slope:80`、`_linear_regression_r_squared:93`、`_ema:110`、`_analyze_market_status:118`、`_daily_avg_prices_last_n:129`、`analyze_by_time:139`、`calculate_stability:337`

依赖：`collections`、`datetime`、`statistics`、`typing`、`utils.money`、`utils.time`

### app/__init__.py

SHA-256：`7eb70257593da06f682a3ddda54a9d260d4fc514f645237f5ca74b08f8da61a6`

符号：包入口，无定义

依赖：无 import

### app/__main__.py

SHA-256：`e25aee7d26261110026fcdecf4b04298629ac98ab451a2a7d156679c3d874d3b`

符号：包入口，无定义

依赖：`app.main`

### app/accounts.py

SHA-256：`461c8705aaa2ff1a3b65d80d60cf6e0cfed29f6ca282f021be6d690b05f7c77e`

符号：`_load:7`、`_save:20`、`list_accounts:26`、`get_current_id:29`、`get_current_account:31`、`get_account:37`、`add_account:39`、`update_account:57`、`delete_account:82`、`set_current:92`、`replace_all:99`、`get_profile_dir:105`

依赖：`json`、`pathlib`、`typing`、`uuid`

### app/api.py

SHA-256：`4b33a78bc87bcbf861b7e181d46bb91034ad0edc058a45dc43efc0259ff94a5b`

符号：`_start_background_workers:28`、`_lifespan:45`、`api_runtime:50`、`api_tasks:54`、`api_task_detail:58`、`shutdown_system:65`、`_do_shutdown:66`

依赖：`app.database`、`app.routes`、`app.runtime_env`、`app.services.task_queue`、`app.services.workers`、`app.state`、`contextlib`、`fastapi`、`os`、`signal`、`sys`、`threading`、`time`

路由：
- `GET /api/runtime → api_runtime:50`
- `GET /api/tasks → api_tasks:54`
- `GET /api/tasks/{task_id} → api_task_detail:58`
- `POST /api/system/shutdown → shutdown_system:65`

### app/config_loader.py

SHA-256：`e8e41507867b7535dbdac822f067e3e4ab5218e509f03dd78db1eb30bd83780f`

符号：`_config_file_revision:21`、`_load_stable_app_config:27`、`_invalidate_config_cache:37`、`get_steam_credentials:43`、`get_buff_credentials:45`、`update_steam_creds:47`、`update_buff_creds:49`、`get_buff_credentials_generation:55`、`load_app_config_validated:61`、`save_app_config_validated:85`、`update_app_config_validated:92`

依赖：`app.config_schema`、`config`、`copy`、`threading`、`time`

### app/config_schema.py

SHA-256：`23dd4462fbe084fd442cffc4f0172f23103bca34600507445c021fcd8c8fe35b`

符号：`merge:130`、`_coerce_bool:140`、`_validate_ranges:155`、`get_app_config:227`、`validate_and_fill:229`

依赖：`copy`、`typing`、`warnings`

### app/database.py

SHA-256：`acc89d674a1fc94cf377e522f3849242b9600752f7367ee63bf0a18b1e36764a`

符号：`_compute_wilson_score:13`、`Purchase:25`、`Sale:44`、`ItemNameId:51`、`SteamDealGame:54`、`get_engine:101`、`get_session:115`、`init_db:117`、`_purchase_from_dict:170`、`_sale_from_dict:188`、`_purchase_to_dict:196`、`_sale_to_dict:227`、`migrate_from_json:237`、`db_append_purchase:274`、`db_get_purchases:278`、`db_append_sale:282`、`db_get_sales:286`、`db_clear_transactions:290`、`db_replace_transactions:296`、`db_delete_purchase:306`、`db_delete_sale:315`、`db_update_purchase:323`、`db_update_purchase_by_id:336`、`db_delete_purchase_by_id:350`、`db_update_sale:361`、`db_delete_sale_by_id:373`、`db_get_item_nameid:384`、`db_set_item_nameid:390`、`_game_row_to_dict:405`、`db_upsert_steam_deal:424`、`db_get_steam_deals:443`、`db_get_steam_deals_count:490`、`db_get_steam_deals_last_update:502`、`db_clear_steam_deals:509`、`db_delete_steam_deals_except:514`、`db_get_steam_deals_price_snapshot:534`、`db_get_steam_deals_review_snapshot:551`、`db_get_steam_deals_by_app_ids:559`

依赖：`json`、`math`、`os`、`pathlib`、`sqlalchemy`、`sqlmodel`、`threading`、`typing`

### app/gift_engine.py

SHA-256：`f29154fe7173ada018eb2edc37663650ced0029d88f17a01673d395eba9af270`

符号：`_build_cookies:12`、`get_friend_list:18`、`_normalize_country_code:114`、`_find_country_code:121`、`_extract_store_user_config:139`、`_extract_country_code:155`、`get_base_auth_status:170`、`get_wallet_balance:191`、`extract_appid_from_url:282`、`get_all_available_editions:285`、`_encode_varint:358`、`_build_addcart_payload:365`、`_build_modify_payload:374`、`_build_remove_payload:383`、`_grpc_request:386`、`_get_cart_items:416`、`_steamid64_to_accountid:443`、`run_gift_flow:449`、`_step:460`、`_fail:462`、`_success:464`、`_do_checkout:519`

依赖：`app.state`、`base64`、`bs4`、`html`、`json`、`re`、`requests`、`steam.session`、`time`、`urllib3`、`utils.proxy_manager`

### app/inventory_cs2.py

SHA-256：`21bfad305ccd58b503f567de82239b2e76498c09a60d7d94ef438881f1da3651`

符号：`_safe_iso:8`、`_parse_cooldown:15`、`scan_cs2_inventory:35`

依赖：`config`、`datetime`、`re`、`steam.inventory`、`steam.session`、`time`、`typing`

### app/main.py

SHA-256：`e4c0612c360a9ccd53392e7d6155ebf384076b1bdc9b314fe9e7b9a60f641bed`

符号：`check_runtime_dependencies:27`、`_ensure_disclaimer:43`、`_load_webview:67`、`__getattr__:75`、`run_server:82`、`_wait_for_server_thread:92`、`_probe_host:100`、`_wait_for_server_ready:104`、`main:118`

依赖：`__future__`、`app.api`、`app.runtime_env`、`os`、`pathlib`、`socket`、`sys`、`threading`、`time`、`typing_extensions`、`uvicorn`、`webbrowser`、`webview`

### app/notify.py

SHA-256：`da3a3e22517618ee6081f2d14f8de984edf3e699b75dc59269737c96c954151c`

符号：`send_pushplus:9`、`notify_manual_intervention_required:23`、`build_payment_notify_content:54`、`wait_email_command:89`、`compute_holdings_stats:180`、`build_holdings_report_content:200`

依赖：`app.config_loader`、`email`、`email.header`、`imaplib`、`requests`、`threading`、`time`、`typing`、`urllib.parse`

### app/pipeline.py

SHA-256：`9a8c463c23478eaf8acab628f9ab6b8cb670672a77360c78ef82042797d67f27`

符号：`_resolve_schedule_timezone:63`、`_now_in_schedule_timezone:67`、`_is_in_time_window:71`、`_fetch_and_filter_deals:88`、`_process_deals_for_target:124`、`buff_log:143`、`time_window_is_open:146`、`on_entering_payment:185`、`_run_pipeline:283`、`time_window_is_open:338`、`PipelineMaintenanceBlocked:541`、`is_shutdown_pending:545`、`mark_shutdown_pending:549`、`exclusive_pipeline_maintenance:555`、`is_pipeline_running:593`、`_run_pipeline_guarded:598`、`get_pipeline_start_blocker:627`、`start_pipeline:689`

依赖：`__future__`、`app.config_loader`、`app.config_schema`、`app.pipeline_context`、`app.pipeline_steps`、`app.sell_pipeline`、`app.services.analysis_client`、`app.services.buff_auth`、`app.services.buff_checkout_guard`、`app.services.buff_client`、`app.services.iflow_client`、`app.services.steam_client`、`app.state`、`app.strategy_engine`、`buff`、`contextlib`、`datetime`、`pathlib`、`steamdt.models`、`sys`、`threading`、`time`、`utils.delay`、`utils.money`、`utils.network_check`、`utils.proxy_manager`、`utils.time`、`uuid`

### app/pipeline_context.py

SHA-256：`bb12da81b275792bef1555a45f38ee53e552a455c2b8601bbd4919cdf4e67e0b`

符号：`PipelineContext:4`、`__init__:5`、`log:10`、`debug:13`、`set_status:17`、`is_stop_requested:20`、`wait_retry:23`

依赖：`time`

### app/pipeline_steps.py

SHA-256：`fe59ae190f54e9f552668ef24f03b95c1a21c264742506f6d7183be6459447fb`

符号：`_currency_amount:41`、`_affordable_quantity:48`、`_PurchaseAttemptStatus:62`、`PurchaseFlowHalted:73`、`__init__:76`、`PurchaseWriteResultUnknown:88`、`PurchaseOrderCreatedPending:92`、`PurchaseCoolingDown:96`、`_mark_committed:100`、`_validate_unique_batch_matches:113`、`_PurchaseAttempt:158`、`_checkout_credential_identity:166`、`_supports_keyword_argument:185`、`_verify_checkout_session:197`、`_ensure_checkout_identity_unchanged:211`、`_cache_buff_sell_orders:231`、`_buff_orders_cache_is_fresh:237`、`_fetch_steam_sell_data:248`、`_check_buff_price:315`、`_adjust_ref_price_for_daily_high:348`、`_compute_sell_pressure_from_orders:406`、`_compute_sell_pressure:443`、`_fetch_smart_market_price:454`、`_parse_threshold:464`、`_goods_id_from_buff_url:473`、`_log_stability_rejection:483`、`filter_iflow_rows:507`、`_check_sell_pressure_precheck:573`、`_check_max_discount_precheck:595`、`_passes_final_buff_precheck:625`、`_build_buy_strategy_outputs:669`、`_passes_custom_buy_modules:714`、`pick_stable_item:752`、`_do_payment_notify_and_wait:1019`、`_email_waiter:1082`、`_do_batch_wait_finalize_and_append:1095`、`_persist_matches:1143`、`_on_match:1165`、`_do_wait_payment_and_append:1344`、`lock_and_confirm_payment:1439`、`_classify_failure:1656`、`_write_snapshot_is_fresh:1719`、`_time_window_is_open:1726`、`_try_single_buy:1735`、`_on_order_created:1802`、`_try_batch_buy:1992`、`_on_batch_created:2044`、`_finish_attempt:2247`

依赖：`app.config_loader`、`app.notify`、`app.services.analysis_client`、`app.services.buff_auth`、`app.services.buff_checkout_guard`、`app.services.buff_client`、`app.services.iflow_client`、`app.services.steam_client`、`app.state`、`app.strategy_engine`、`buff`、`dataclasses`、`datetime`、`decimal`、`enum`、`inspect`、`re`、`statistics`、`steam.market_orders`、`steam.session`、`threading`、`time`、`typing`、`utils.delay`、`utils.money`、`utils.time`

### app/purchase_limit.py

SHA-256：`c0044fe06f7e675a08ccd4c0c5f7d0f2db5999dd863ad08b8e22a527c5cae2c4`

符号：`calculate_safe_purchase_limit:1`

依赖：无 import

### app/receive_flow.py

SHA-256：`d85c4a2f3b1f72594b759905e430aec1b9b0cec5fe3774ae00162d4055327d3c`

符号：`_cookies_str_to_dict:17`、`fetch_buff_steam_trade:25`、`accept_steam_trade_offer:103`、`_match_purchase_for_item:162`、`try_receive_once:214`、`_do_update:250`

依赖：`app.pipeline`、`app.services.buff_auth`、`app.services.buff_checkout_guard`、`app.state`、`buff`、`collections`、`requests`、`typing`、`urllib3`、`utils.delay`、`utils.proxy_manager`

### app/repair_error_records.py

SHA-256：`888fcfcb57f6503abb20758842867275e3a53ca24a906500bc1721b12bc3ab73`

符号：`_cookies_to_dict:26`、`_load_rate_map:36`、`_currency_code_from_price_text:47`、`_parse_sold_history_page:64`、`_history_total_count:130`、`_fetch_sold_with_names:139`、`_norm_name:171`、`_build_merged:178`、`_record_name_counts:202`、`_source_order:209`、`_pick_candidate:214`、`_has_sale_price:221`、`_candidate_priority:226`、`_candidate_by_assetid:228`、`_apply_candidate:238`、`_reset_unresolved_active_record:259`、`_rebuild_records:272`、`run:322`、`main:415`、`log:416`

依赖：`app.inventory_cs2`、`app.state`、`app.steam_listings`、`bs4`、`collections`、`json`、`pathlib`、`re`、`requests`、`sys`、`time`、`unicodedata`、`urllib.parse`、`urllib3`

### app/routes/__init__.py

SHA-256：`65ca3d77cb330d3ef373540880560198e48a04693c16c7e9685d5b5f76952267`

符号：`register_routes:17`

依赖：`app.routes.accounts`、`app.routes.auth`、`app.routes.config`、`app.routes.gift`、`app.routes.inventory`、`app.routes.pipeline`、`app.routes.proxy`、`app.routes.static`、`app.routes.status`、`app.routes.steam_deals`、`app.routes.strategies`、`app.routes.transactions`、`fastapi`

### app/routes/accounts.py

SHA-256：`206e0baf6edc70649c9cf81bfecd812e10fcbdaf7c2167a64dfc71860fb87bab`

符号：`AccountBody:18`、`AccountUpdateBody:24`、`api_list_accounts:31`、`api_add_account:37`、`api_update_account:47`、`api_delete_account:64`、`api_set_current_account:68`、`api_verify_account:72`

依赖：`app.accounts`、`app.services.account_region`、`app.services.steam_auth`、`app.state`、`fastapi`、`pydantic`、`typing`

路由：
- `GET /api/accounts → api_list_accounts:31`
- `POST /api/accounts → api_add_account:37`
- `PUT /api/accounts/{account_id} → api_update_account:47`
- `DELETE /api/accounts/{account_id} → api_delete_account:64`
- `POST /api/accounts/{account_id}/set_current → api_set_current_account:68`
- `POST /api/accounts/{account_id}/verify → api_verify_account:72`

### app/routes/auth.py

SHA-256：`2443a12b1db3565d91c17b8d4da1e77360a41bcc5f10377b27180e7371341e0f`

符号：`_manual_cookie_required_response:57`、`ReloginFinishBody:67`、`ManualCookieBody:69`、`_normalize_cookie_input:74`、`_normalize_user_agent:88`、`_cookie_value:92`、`_cookie_header_from_browser:101`、`_has_browser_cookie:109`、`_steam_id_from_cookie_str:116`、`_looks_like_browser_launch_error:123`、`_should_retry_browser_launch:138`、`_friendly_browser_launch_error:152`、`_launch_relogin_context:170`、`_maybe_resume_after_buff_cookie_update:214`、`_relogin_worker:235`、`_relogin_start:376`、`_relogin_finish:421`、`_normalize_secret:433`、`_generate_steam_guard_code:435`、`api_auth_steam_relogin_start:455`、`api_auth_steam_relogin_finish:458`、`api_auth_buff_relogin_start:461`、`api_auth_buff_relogin_finish:464`、`api_auth_manual_cookie:467`、`remember_validated_cookie:525`、`api_steam_guard:565`

依赖：`app.accounts`、`app.config_loader`、`app.pipeline`、`app.runtime_env`、`app.services.buff_auth`、`app.services.steam_auth`、`app.state`、`base64`、`fastapi`、`hashlib`、`hmac`、`pathlib`、`playwright.sync_api`、`pydantic`、`re`、`shutil`、`struct`、`tempfile`、`threading`、`time`、`typing`

路由：
- `POST /api/auth/steam/relogin_start → api_auth_steam_relogin_start:455`
- `POST /api/auth/steam/relogin_finish → api_auth_steam_relogin_finish:458`
- `POST /api/auth/buff/relogin_start → api_auth_buff_relogin_start:461`
- `POST /api/auth/buff/relogin_finish → api_auth_buff_relogin_finish:464`
- `POST /api/auth/{relogin_type}/manual_cookie → api_auth_manual_cookie:467`
- `GET /api/steam_guard → api_steam_guard:565`

### app/routes/config.py

SHA-256：`316b7d5e130b42dda38380fe5f1f6cb4e06ccdc5fe03df1f8ed5f5c28a9bddb5`

符号：`_save_credentials_with_auth_lock:22`、`ConfigBody:34`、`ImportFullBody:36`、`api_get_config:43`、`api_save_config:47`、`_api_data_init_unlocked:50`、`_shutdown:170`、`api_data_init:180`、`api_export_full:200`、`api_export_full_download:215`、`api_import_full:247`、`api_holdings_report_send:338`

依赖：`app.accounts`、`app.config_loader`、`app.database`、`app.pipeline`、`app.services.buff_auth`、`app.services.workers`、`app.state`、`config`、`datetime`、`fastapi`、`fastapi.responses`、`json`、`os`、`pathlib`、`pydantic`、`shutil`、`sqlmodel`、`threading`、`time`、`typing`、`utils.time`

路由：
- `GET /api/config → api_get_config:43`
- `POST /api/config → api_save_config:47`
- `POST /api/data/init → api_data_init:180`
- `GET /api/export_full → api_export_full:200`
- `GET /api/export_full/download → api_export_full_download:215`
- `POST /api/import_full → api_import_full:247`
- `POST /api/holdings_report/send → api_holdings_report_send:338`

### app/routes/gift.py

SHA-256：`50daf5e0e41cf44bab6a211ebefaa0a32a7e723293f2a5c3c8687ae385b9befe`

符号：`_new_task:16`、`_append_progress:24`、`_get_task:32`、`_get_cookies_raw:36`、`_get_steam_id:40`、`_check_cookies:44`、`_sync_wallet_info_to_current_account:63`、`api_get_friends:85`、`EditionsBody:98`、`api_get_editions:101`、`GiftBody:114`、`api_gift_send:119`、`_run:126`、`api_gift_task:143`、`api_gift_balance:149`、`api_gift_debug_config:161`

依赖：`app`、`app.accounts`、`app.config_loader`、`app.services.account_region`、`app.services.steam_auth`、`fastapi`、`pydantic`、`threading`、`time`、`typing`、`uuid`

路由：
- `GET /api/gift/friends → api_get_friends:85`
- `POST /api/gift/editions → api_get_editions:101`
- `POST /api/gift/send → api_gift_send:119`
- `GET /api/gift/task/{task_id} → api_gift_task:143`
- `GET /api/gift/balance → api_gift_balance:149`
- `GET /api/gift/debug-config → api_gift_debug_config:161`

### app/routes/inventory.py

SHA-256：`c472104883babee0c31478db73425b7e181e80b25311fd348a3694c116d91a2d`

符号：`_get_steam_smart_price_cny:9`、`_enrich_inventory_with_steam_prices:11`、`_try_steam_auto_relogin:35`、`api_inventory:39`、`api_market_prices:104`

依赖：`app.config_loader`、`app.inventory_cs2`、`app.pipeline`、`app.services.steam_auth`、`app.shared_market`、`app.state`、`fastapi`、`time`

路由：
- `GET /api/inventory → api_inventory:39`
- `GET /api/market-prices → api_market_prices:104`

### app/routes/pipeline.py

SHA-256：`ef29b75b769b416d0e1f2f485be4c347628f0aedccf69a3a2f34e2216fcaf375`

符号：`ConfigBody:7`、`api_pipeline_start:12`、`api_buff_checkout_guard:53`、`api_pipeline_stop:62`

依赖：`app.pipeline`、`app.state`、`fastapi`、`pydantic`

路由：
- `POST /api/pipeline/start → api_pipeline_start:12`
- `GET /api/pipeline/buff_checkout_guard → api_buff_checkout_guard:53`
- `POST /api/pipeline/stop → api_pipeline_stop:62`

### app/routes/proxy.py

SHA-256：`30df4a3d5f23799c51d28f113e15dbf82a55f0342f03157fa4d7b9fb530dc7d6`

符号：`ProxyEntry:10`、`ProxyPoolConfig:15`、`ProxyPoolBody:22`、`api_get_proxy_config:25`、`api_save_proxy_config:29`、`api_test_proxies:50`、`api_clear_proxies:81`、`api_fetch_webshare:95`

依赖：`app.config_loader`、`app.state`、`concurrent.futures`、`fastapi`、`pydantic`、`requests`、`time`、`typing`、`utils.proxy_manager`

路由：
- `GET /api/proxy/config → api_get_proxy_config:25`
- `POST /api/proxy/config → api_save_proxy_config:29`
- `POST /api/proxy/test → api_test_proxies:50`
- `POST /api/proxy/clear → api_clear_proxies:81`
- `POST /api/proxy/webshare → api_fetch_webshare:95`

### app/routes/static.py

SHA-256：`a0225eb8182c0f300832fa88408096f44c5448e56c252d4cc638f1faa99a4603`

符号：`_web_file_response:10`、`_safe_web_file:19`、`index:30`、`static_or_index:36`

依赖：`fastapi`、`fastapi.responses`、`pathlib`、`typing`

路由：
- `GET / → index:30`
- `GET /{path:path} → static_or_index:36`

### app/routes/status.py

SHA-256：`fdd7fda876675bb2cc0f1a02cfcab426de3b5eb14ab1810dc34cee958ff74aef`

符号：`ConfirmBody:22`、`api_status:25`、`api_log:32`、`api_log_clear:35`、`api_log_export:39`、`fmt_time:48`、`api_plan:62`、`api_pending_payment:65`、`api_confirm_payment:68`

依赖：`app.config_loader`、`app.state`、`config`、`fastapi`、`pathlib`、`pydantic`、`utils.time`

路由：
- `GET /api/status → api_status:25`
- `GET /api/log → api_log:32`
- `POST /api/log/clear → api_log_clear:35`
- `POST /api/log/export → api_log_export:39`
- `GET /api/plan → api_plan:62`
- `GET /api/pending_payment → api_pending_payment:65`
- `POST /api/confirm_payment → api_confirm_payment:68`

### app/routes/steam_deals.py

SHA-256：`2fb05392f67dd026428e6acb10ccfa3028c3d1a7dccf47ed86d87934825863ff`

符号：`_load_exchange_rates:35`、`_get_rates:45`、`_invalidate_sort_cache:58`、`_build_sort_index:61`、`_parse_price:129`、`_to_cny:165`、`api_get_steam_deals:184`、`api_exchange_rates:255`、`api_fetch_steam_deals:260`、`api_pause_steam_deals:289`、`api_steam_deals_status:297`、`api_generate_deal_card:320`

依赖：`app.config_loader`、`app.database`、`app.services.deal_cards`、`app.services.steam_deals`、`fastapi`、`json`、`math`、`pathlib`、`re`、`sqlmodel`、`threading`、`time`、`typing`

路由：
- `GET /api/steam-deals → api_get_steam_deals:184`
- `GET /api/steam-deals/exchange-rates → api_exchange_rates:255`
- `POST /api/steam-deals/fetch → api_fetch_steam_deals:260`
- `POST /api/steam-deals/pause → api_pause_steam_deals:289`
- `GET /api/steam-deals/status → api_steam_deals_status:297`
- `POST /api/steam-deals/generate-card/{app_id} → api_generate_deal_card:320`

### app/routes/strategies.py

SHA-256：`87b3de218c8371c8c0f17933dbfb3ab07e9876df652d681acfff76e3cf8a00ff`

符号：`StrategyBody:20`、`ActivateBody:24`、`ImportBody:28`、`ModuleImportBody:32`、`SimulateBody:36`、`_error:41`、`api_strategies:46`、`api_save_strategy:51`、`api_delete_strategy:59`、`api_activate_strategy:68`、`api_import_strategy:76`、`api_export_strategy:84`、`api_import_strategy_module:92`、`api_simulate_strategy:100`

依赖：`app.strategy_engine`、`fastapi`、`pydantic`

路由：
- `GET /api/strategies → api_strategies:46`
- `POST /api/strategies → api_save_strategy:51`
- `DELETE /api/strategies/{strategy_id} → api_delete_strategy:59`
- `POST /api/strategies/{strategy_id}/activate → api_activate_strategy:68`
- `POST /api/strategies/import → api_import_strategy:76`
- `GET /api/strategies/{strategy_id}/export → api_export_strategy:84`
- `POST /api/strategy-modules/import → api_import_strategy_module:92`
- `POST /api/strategies/simulate → api_simulate_strategy:100`

### app/routes/transactions.py

SHA-256：`b23d85171fd1aa86964c1fe9d450a9878023a6ae34ad2e028037e06cdedd8d25`

符号：`AddPurchaseBody:29`、`TransactionUpdateBody:36`、`_name_from_steam_link:53`、`_get_steam_smart_price_cny:66`、`_fetch_steam_lowest_cny:68`、`_enrich_purchases_with_current_prices:74`、`api_purchases:94`、`api_add_purchase:97`、`api_transactions:124`、`api_delete_transaction:172`、`api_update_transaction:181`、`api_stats:228`、`api_delist_purchase:261`、`log_fn:297`、`api_sync_sold_from_history:389`、`log_fn:391`、`api_repair_error_records:408`、`log_fn:410`

依赖：`app.config_loader`、`app.repair_error_records`、`app.shared_market`、`app.state`、`app.steam_delist`、`app.sync_sold`、`fastapi`、`pydantic`、`steam.client`、`time`、`typing`、`utils.proxy_manager`

路由：
- `GET /api/purchases → api_purchases:94`
- `POST /api/purchase → api_add_purchase:97`
- `GET /api/transactions → api_transactions:124`
- `DELETE /api/transaction → api_delete_transaction:172`
- `PUT /api/transaction → api_update_transaction:181`
- `GET /api/stats → api_stats:228`
- `POST /api/purchase/{idx}/delist → api_delist_purchase:261`
- `POST /api/sync_sold_from_history → api_sync_sold_from_history:389`
- `POST /api/repair_error_records → api_repair_error_records:408`

### app/runtime_env.py

SHA-256：`d4d849da3d2caf14bbe65df1118a331b4a983afc1ac64b30d974bfe89de6e7c8`

符号：`RuntimeProfile:16`、`as_dict:27`、`_clean:31`、`parse_bool:35`、`env_bool:46`、`_env_value:51`、`has_graphical_display:60`、`get_runtime_profile:71`、`get_bind_host:127`、`get_port:135`、`get_local_url:146`、`get_external_url_hint:151`

依赖：`__future__`、`dataclasses`、`os`、`sys`、`typing`

### app/sell_pipeline.py

SHA-256：`ad6320f33ae87c837b9eec47da0a880fb2dc7a3247372a4ef5c85e85ce2501d6`

符号：`_steam_latest_price_and_trend:34`、`_load_rate_map:74`、`_record_listing_success:88`、`_resolve_steam_session:108`、`_get_inventory:127`、`_build_listing_plan:155`、`_find_buy_record:380`、`_listing_response_body_preview:414`、`_parse_listing_response:448`、`_listing_response_message:467`、`_listing_response_is_success:474`、`_submit_listings:486`、`_do_list:512`、`_auto_confirm_listings:576`、`_run_sell_phase_impl:604`、`_run_sell_phase:713`、`run_sell_phase_on_inventory_update:727`

依赖：`app.accounts`、`app.config_loader`、`app.config_schema`、`app.inventory_cs2`、`app.pipeline_context`、`app.services.account_region`、`app.services.steam_auth`、`app.services.steam_client`、`app.state`、`app.steam_confirm`、`app.steam_listings`、`app.strategy_engine`、`collections`、`datetime`、`json`、`pathlib`、`re`、`steam.market`、`steam.market_orders`、`steam.session`、`threading`、`time`、`typing`、`utils.delay`、`utils.money`、`utils.time`、`utils.trend`

### app/services/__init__.py

SHA-256：`b7f31b41f81376f7be769096888e1f6378998c3f9dfca809f40b3a7fd948ee1f`

符号：包入口，无定义

依赖：`app.services.analysis_client`、`app.services.buff_client`、`app.services.iflow_client`、`app.services.retry`、`app.services.steam_client`

### app/services/account_region.py

SHA-256：`e37d18cf4af4a75048e0af7231846e3eb4cc72d23604a3b5c7ada55a9e690f5c`

符号：`_short_error:48`、`_region_from_settlement_currency:54`、`refresh_account_region_currency:60`

依赖：`app.accounts`、`app.config_loader`、`app.gift_engine`、`app.services.steam_auth`、`time`、`typing`

### app/services/analysis_client.py

SHA-256：`8bc852e2dac331db6aaf62000122bc61cfcd209db369af576b7ad4b3a43477a0`

符号：`StabilityAnalyzer:4`、`__init__:5`、`analyze:7`

依赖：`analysis`、`typing`、`utils.money`

### app/services/buff_auth.py

SHA-256：`c563d8f08881ecc1669373d3334fb14f4b1ba5214b8cbe6cde3120464477a3aa`

符号：`get_buff_auth_lock:66`、`buff_credential_replacement_block_reason:76`、`browser_buff_verification_allowed:116`、`_global_policy_and_account_key:136`、`_evaluate_browser_verification_with_global_pacing:148`、`_clear_account_circuit_preserving_pacing:201`、`_trip_browser_verification:233`、`clear_buff_request_policy_after_verification:260`、`read_buff_browser_user_agent:295`、`detect_buff_challenge:305`、`_response_has_challenge:336`、`verify_buff_browser_session:341`、`verify_manual_buff_credentials:468`、`manual_buff_probe_allowed:541`、`try_buff_auto_relogin:572`、`_notify_buff_expired:593`、`_try_buff_auto_relogin_impl:605`

依赖：`app.config_loader`、`app.notify`、`app.pipeline`、`app.services.buff_checkout_guard`、`app.state`、`buff`、`buff.request_policy`、`json`、`pathlib`、`playwright.sync_api`、`threading`、`time`、`typing`

### app/services/buff_checkout_guard.py

SHA-256：`32a227e19d48a6f22ca2072dbda470a020e5396aa033248868a39fdcbbd45f3a`

符号：`BuffCheckoutGuardActive:52`、`__init__:55`、`BuffCheckoutGuardMismatch:70`、`_invalid_journal_state:74`、`_reject_non_finite_json:84`、`_reject_duplicate_json_keys:88`、`_valid_timestamp:97`、`_journal_is_valid:106`、`_activity_lock_path:141`、`_lock_activity_file:145`、`_unlock_activity_file:176`、`buff_activity_guard:189`、`_json_safe:229`、`_read_raw_locked:246`、`_atomic_write_locked:269`、`get_unresolved_checkout:296`、`begin_checkout:304`、`_assert_expected_intent:345`、`update_checkout:361`、`resolve_checkout:382`、`acknowledge_checkout:409`

依赖：`__future__`、`contextlib`、`copy`、`errno`、`fcntl`、`json`、`math`、`msvcrt`、`os`、`pathlib`、`tempfile`、`threading`、`time`、`typing`、`uuid`

### app/services/buff_client.py

SHA-256：`23243f654ea320041208488dc07182ef804b417f8e02f8140a8a07fe4e56cc8a`

符号：`count_lowest_price_orders:20`、`first_order_at_price:40`、`BuffClient:51`、`__init__:65`、`_as_generation:92`、`_new_buyer:98`、`_ensure_current_buyer:108`、`_persist_rotated_cookies:132`、`get_credential_identity:146`、`_run:172`、`close:186`、`get_sell_orders:192`、`verify_session:195`、`operation:196`、`get_steam_trades:204`、`get_goods_steam_price_cny:207`、`ask_seller_to_send:214`、`lock_and_get_pay_url:221`、`prepare_single_buy:242`、`operation:251`、`try_batch_buy:282`、`batch_buy_find_and_finalize:301`、`operation:313`、`create_buff_client_from_config:396`、`persist_rotated_cookies:402`

依赖：`app.config_loader`、`app.services.buff_auth`、`buff`、`hashlib`、`logging`、`threading`、`typing`

### app/services/deal_cards.py

SHA-256：`5caf7cd5179da0f91f74cf86aa94a29c648e9266147e0ee4ea88e8b637911c0f`

符号：`_load_font:54`、`_load_currency_font:77`、`_download_banner:100`、`_get_flag:122`、`draw_rounded_rect:148`、`draw_antialiased_circle:164`、`get_text_bbox:180`、`get_text_size:189`、`draw_text_middle:198`、`draw_text_left:206`、`parse_price_to_rmb:214`、`format_original_price:263`、`wrap_text:268`、`generate_card:289`

依赖：`PIL`、`app.config_loader`、`app.database`、`argparse`、`io`、`json`、`os`、`pathlib`、`re`、`requests`、`sqlmodel`、`sys`、`time`、`typing`、`utils.proxy_manager`、`utils.time`

### app/services/iflow_client.py

SHA-256：`60ab0cdcad97bb36e1546adddc04c463a4d65bc3cd452da00e598ee0051be4a5`

符号：`IflowClient:26`、`__init__:33`、`fetch:36`、`steamdt_proxy_provider:74`、`fetch_iflow_rows:86`

依赖：`math`、`steamdt.fetcher`、`steamdt.models`、`typing`、`utils.proxy_manager`

### app/services/retry.py

SHA-256：`4d9b54338ae504bc15deedf15eb30d538d494b0f83d87fea07a900f4e981d493`

符号：`with_retry:8`、`decorator:15`、`inner:17`

依赖：`functools`、`logging`、`time`、`typing`、`utils.delay`

### app/services/steam_auth.py

SHA-256：`4914c49d29b8c38f477ed3ca3b0eebb27f3bb9b803b5f3a33f8f35f2ef3ddd43`

符号：`SteamAuthTokenPending:34`、`SteamCredentialsRejected:38`、`_verify_steam_cookies_valid:42`、`fetch_steam_profile_via_api:93`、`_get_shared_secret:165`、`_build_steam_guard_dict:174`、`_short_error_detail:199`、`_is_explicit_credentials_error_text:213`、`_is_explicit_credentials_error:237`、`_steam_login_response_error_text:247`、`_is_pending_steam_session_exception:268`、`_classify_steam_login_exception:288`、`_poll_steam_refresh_token:339`、`_steampy_pool_sessions_with_retry:404`、`_do_steampy_login_once:414`、`_steam_request:438`、`_update_steam_guard_with_validation:453`、`_get_steam_login_proxy_manager:519`、`_do_steampy_login:525`、`steam_id_from_cookie_str:621`、`_extract_creds_from_cookie_dict:635`、`try_steam_auto_relogin:643`、`_try_steam_auto_relogin_impl:654`、`verify_steam_auto_login:745`

依赖：`app.accounts`、`app.config_loader`、`app.notify`、`app.state`、`json`、`re`、`requests`、`requests.utils`、`steampy.client`、`steampy.login`、`threading`、`time`、`typing`、`urllib3`、`utils.proxy_manager`

### app/services/steam_client.py

SHA-256：`5868c04cab37700f04a4d1267827561f54502edf8f52f3fe47a564539ade7b2e`

符号：`SteamClient:14`、`__init__:15`、`fetch_history:24`、`_fetch_history_impl:43`、`market_hash_name_from_listing_url:77`、`create_steam_client:79`

依赖：`app.config_loader`、`app.services.retry`、`app.state`、`steam.client`、`time`、`typing`、`utils.delay`、`utils.proxy_manager`

### app/services/steam_deals.py

SHA-256：`fd6ac3c4cb3606f2ad488da625aeaf7cc486f56dd5ec473fee1f31af33591282`

符号：`get_fetch_state:42`、`_update_state:45`、`request_pause:48`、`_pause_requested:57`、`_clear_pause_request:59`、`_load_checkpoint:63`、`_save_checkpoint:91`、`_clear_checkpoint:104`、`get_resume_state:109`、`_build_valid_proxies:139`、`_test:166`、`_fetch_with_proxy:187`、`_get_discounted_appids:202`、`_fetch_region_data:349`、`_fetch_historical_low:381`、`_get_deal_status:397`、`_process_single_game:412`、`run_fetch:494`、`_ordered_done:582`、`_save_processing_checkpoint:597`、`_submit_more:609`

依赖：`app.config_loader`、`app.database`、`concurrent.futures`、`json`、`pathlib`、`random`、`re`、`requests`、`threading`、`time`、`typing`、`urllib3`

### app/services/task_queue.py

SHA-256：`a4afe5afbfb99523fe45e228169b13a3100f57c599bfd2e67d5f67f7409730f7`

符号：`TaskStatus:15`、`TaskInfo:22`、`TaskQueue:33`、`__init__:42`、`submit:47`、`get_task:87`、`list_tasks:93`、`active_count:97`、`shutdown:100`、`_run_with_retries:102`、`_trim_history:132`、`_info_to_dict:144`、`get_task_queue:159`

依赖：`concurrent.futures`、`dataclasses`、`enum`、`threading`、`time`、`traceback`、`typing`、`uuid`

### app/services/workers.py

SHA-256：`241de0ec077e2d3b54e368c2d8a526e6ef1691e43ecb4694b9f9efd7800cb1aa`

符号：`_worker_alert:35`、`_load_last_pl_pct:52`、`_save_last_pl_pct:62`、`_enrich_purchases_with_current_prices:69`、`run_holdings_report_once:88`、`_build_out:99`、`_enriched_holdings:108`、`holdings_report_worker:139`、`_fetch_exchange_rates:165`、`_save_exchange_rates:193`、`exchange_rate_worker:206`、`_buff_background_request_is_safe:243`、`receive_worker:261`、`listing_check_worker:333`、`_currency_code_from_price_text:403`、`_detect_account_currency_from_history:420`、`_sync_account_profile_and_region:466`、`sync_account_region_worker:492`、`_session_keepalive_enabled:530`、`_session_keepalive_is_safe:534`、`session_keepalive_worker:550`

依赖：`app.accounts`、`app.config_loader`、`app.inventory_cs2`、`app.notify`、`app.pipeline`、`app.receive_flow`、`app.sell_pipeline`、`app.services.account_region`、`app.services.buff_auth`、`app.services.buff_checkout_guard`、`app.services.buff_client`、`app.services.steam_auth`、`app.shared_market`、`app.state`、`app.steam_listings`、`bs4`、`buff`、`datetime`、`json`、`pathlib`、`requests`、`time`、`typing`、`utils.proxy_manager`

### app/shared_market.py

SHA-256：`55b15f8a17d67c3cfda8f6032a5be3edc9be0422b4f042e1a4eecaaafc7854c5`

符号：`get_steam_smart_price_cny:9`、`batch_fetch_prices:28`、`_fetch_one:42`

依赖：`app.config_loader`、`concurrent.futures`、`steam.market_orders`、`steam.session`、`typing`

### app/state.py

SHA-256：`26e96a17d43234a3b92a6e9c4a6eab0c7c3e9ea44b645107f33f39f29d3466b0`

符号：`State:22`、`__init__:41`、`append_purchase:61`、`get_purchases:63`、`append_sale:65`、`get_sales:67`、`clear_transactions:69`、`reload_transactions:71`、`replace_transactions:73`、`delete_purchase:75`、`delete_sale:77`、`update_purchase:79`、`update_purchase_by_id:81`、`delete_purchase_by_id:83`、`update_sale:85`、`set_status:87`、`get_status:98`、`is_steam_background_allowed:113`、`set_buff_auth_expired:118`、`set_buff_verification_required:124`、`log:130`、`get_log:141`、`set_pending_payment:147`、`get_pending_payment:150`、`wait_payment_confirm:153`、`confirm_payment:171`、`request_stop:175`、`clear_stop:180`、`is_stop_requested:183`、`set_plan:186`、`get_plan:189`、`set_inventory:192`、`get_inventory:195`、`clear_log:198`、`replace_log:201`、`get_state:218`、`set_state:224`、`set_status:228`、`get_status:230`、`is_steam_background_allowed:232`、`set_buff_auth_expired:234`、`set_buff_verification_required:236`、`log:238`、`get_log:240`、`set_pending_payment:242`、`get_pending_payment:244`、`wait_payment_confirm:246`、`confirm_payment:248`、`request_stop:250`、`clear_stop:252`、`is_stop_requested:254`、`set_plan:256`、`get_plan:258`、`append_purchase:260`、`get_purchases:262`、`append_sale:264`、`get_sales:266`、`reload_transactions:268`、`clear_transactions:270`、`replace_transactions:272`、`delete_purchase:274`、`delete_purchase_by_id:276`、`delete_sale:278`、`delete_sale_by_id:280`、`update_purchase:282`、`update_purchase_by_id:284`、`update_sale:286`、`set_inventory:288`、`get_inventory:290`、`clear_log:292`、`replace_log:294`

依赖：`app.database`、`collections`、`json`、`pathlib`、`threading`、`time`、`typing`

### app/steam_confirm.py

SHA-256：`3781dddc6ef481f0041bcb70804881e73071f869ae7ef62d980d692d69b068f1`

符号：`_cookies_to_dict:11`、`SteamConfirmer:20`、`__init__:21`、`_signature:40`、`get_confirmations:47`、`accept_all:73`、`auto_confirm_once:106`

依赖：`base64`、`hashlib`、`hmac`、`requests`、`struct`、`time`、`typing`、`urllib.parse`、`urllib3`

### app/steam_delist.py

SHA-256：`8f71efcfd000fd32ee4a13104677f809d25c1f60c2df9e75882f0fd23de00c0b`

符号：`_cookies_to_dict:22`、`_get_mylistings_api:32`、`extract_items:47`、`_extract_js_var:90`、`_get_asset_class_instance_from_market_page:123`、`_get_assetids_by_class_instance:144`、`delist_item:183`

依赖：`config`、`json`、`re`、`requests`、`steam.inventory`、`steam.session`、`time`、`typing`、`urllib3`、`utils.delay`

### app/steam_listings.py

SHA-256：`8d1a5ddb1484fce60ed957f560fab20235ad35fa5a54b29c5e2c612fe477daeb`

符号：`_get_with_retry:21`、`_cookies_to_dict:45`、`_extract_js_var:55`、`_debug:91`、`_parse_assets_730_2:94`、`_parse_sell_listings_from_html:106`、`fetch_my_listings:146`、`fetch_my_history_sold:194`、`_currency_code_from_price_text:242`

依赖：`bs4`、`json`、`pathlib`、`re`、`requests`、`time`、`typing`、`urllib3`、`utils.proxy_manager`

### app/strategy_engine.py

SHA-256：`21a4e93e08eef7bca0909ffdbadf5a4de709f4473fefcccb7070e221d8a7e4a5`

符号：`StrategyError:607`、`_read_json:611`、`_write_json:622`、`_strategy_store:628`、`_save_strategy_store:634`、`_module_store:638`、`_save_module_store:644`、`_sanitize_value:648`、`_slugify:661`、`_custom_id:666`、`_system_by_id:670`、`_user_module_by_id:674`、`_clean_module_id_list:683`、`_clean_requires_any:689`、`_clean_logic:700`、`_coerce_bool:705`、`_clean_stage_list:722`、`_clean_conditions:737`、`_has_user_code_fields:765`、`_is_declarative_user_module:769`、`_normalize_user_module_kind:778`、`_normalize_max_instances:785`、`_apply_module_defaults:792`、`_module_by_id:817`、`list_custom_strategies:834`、`list_strategies:838`、`get_strategy:842`、`_legacy_sell_id:849`、`get_active_strategy_ids:857`、`_normalize_step:869`、`normalize_strategy:878`、`_validate_step_params:906`、`_validate_user_module_contract:954`、`validate_strategy:974`、`_system_strategy_with_submitted_params:1092`、`_save_system_strategy_params:1115`、`save_strategy:1136`、`delete_strategy:1151`、`activate_strategy:1175`、`import_strategy:1198`、`export_strategy:1206`、`import_user_module:1218`、`_enabled_steps:1282`、`_current_param_values:1286`、`_hydrate_strategy_params:1365`、`_step_params:1387`、`_enabled_module_ids:1394`、`_derive_sell_strategy_number:1398`、`apply_strategy_to_config:1409`、`is_strategy_module_enabled:1525`、`get_strategy_module_params:1532`、`_available_data_catalog:1541`、`_get_nested:1550`、`_resolve_output_path:1569`、`_resolve_path:1582`、`_as_number:1597`、`_compare_values:1606`、`_evaluate_condition:1648`、`_evaluate_declarative_module:1669`、`_module_stage_matches:1698`、`evaluate_strategy_runtime_modules:1705`、`_sample_strategy_context:1744`、`_simulate_builtin_step:1765`、`get_strategy_payload:1781`、`simulate_strategy:1803`

依赖：`app.config_loader`、`config`、`copy`、`json`、`pathlib`、`re`、`time`、`typing`

内置策略模块（参数仅列字段，不读取运行配置）：

| ID | 名称 | 参数 |
| --- | --- | --- |
| `buy.steamdt_top_n` | SteamDT 取前 N 条 | iflow_top_n |
| `buy.exclude_keywords` | 关键词排除 | exclude_keywords |
| `buy.basic_candidate_filter` | 基础候选校验 |  |
| `buy.buff_realtime_price` | Buff 实时卖单 |  |
| `buy.steam_sell_depth` | Steam 卖单深度 |  |
| `guard.max_discount` | 最高折扣 | max_discount |
| `guard.sell_pressure` | 卖压保护 | sell_pressure_orders_n, sell_pressure_threshold |
| `guard.history_stability` | 历史稳定性（兼容） | days, cv_threshold, r2_threshold, min_daily_trades, price_percentile_ceil, r2_rising_threshold, slope_pct_ceil, ma_deviation_ceil, last_price_ma30_ceil, slope_stable_floor, price_percentile_ceil_rising, use_vwap |
| `guard.history_data_window` | 历史数据窗口 | days, min_daily_trades, use_vwap |
| `guard.volatility_cv` | 波动率过滤 | cv_threshold |
| `guard.trend_quality` | 趋势质量过滤 | r2_threshold, r2_rising_threshold, slope_pct_ceil, slope_stable_floor |
| `guard.price_position` | 价格位置过滤 | price_percentile_ceil, price_percentile_ceil_rising, ma_deviation_ceil, last_price_ma30_ceil |
| `guard.safe_purchase_limit` | 安全采购上限（兼容） | safe_purchase_hard_qty_cap, safe_purchase_liquidity_ratio, safe_purchase_low_price_threshold, safe_purchase_low_price_penalty, safe_purchase_low_price_hard_cap |
| `guard.purchase_hard_cap` | 单品硬数量上限 | safe_purchase_hard_qty_cap |
| `guard.purchase_liquidity_cap` | 流动性采购上限 | safe_purchase_liquidity_ratio |
| `guard.low_price_purchase_guard` | 低价采购惩罚 | safe_purchase_low_price_threshold, safe_purchase_low_price_penalty, safe_purchase_low_price_hard_cap |
| `guard.held_same_item_guard` | 同名持仓扣减 |  |
| `guard.target_balance` | 目标余额保护 | target_balance |
| `action.buff_lock_pay` | Buff 锁单付款 |  |
| `sell.sellable_inventory_filter` | 可出售库存 |  |
| `guard.max_listings_per_item` | 同名在售上限 | max_listings_per_item |
| `pricing.steam_wall_gap` | 墙+断层一体定价 | sell_price_wall_volume, sell_price_max_ignore_volume, sell_price_offset |
| `pricing.steam_wall_price` | 价格墙定价 | sell_price_wall_volume, sell_price_max_ignore_volume |
| `pricing.price_offset` | 上架价补偿 | sell_price_offset |
| `guard.rising_trend_wait` | 上涨趋势等待 | sell_trend_days |
| `guard.profit_ratio` | 利润比例保护 | profit_ratio_multiplier |
| `action.pause_auto_sell` | 暂停自动出售 |  |
| `action.steam_list` | Steam 上架 |  |

### app/sync_sold.py

SHA-256：`17a9500fcd72373a51d38e72423b0cd3df45e2b8984d6e5a4f2343e486668fbc`

符号：`_fill_assetid_from_inventory:3`、`run_sync_sold_from_history:27`

依赖：`app.config_loader`、`app.inventory_cs2`、`app.state`、`app.steam_listings`、`time`、`typing`

### buff/__init__.py

SHA-256：`faa86ddb39d0f55a89ee9ba2ff9a042666467bb8b3d1475c7927e3c4e8901504`

符号：包入口，无定义

依赖：`.buyer`、`.request_policy`

### buff/buyer.py

SHA-256：`5ef4cc5e470c5ee96c6ce3938344e483a3c332ca19d15e1561179e18cd5cb59e`

符号：`_is_auth_error:37`、`_is_definitive_login_rejection:56`、`_timezone_offset_dst_ms:71`、`_history_item_is_awaiting_payment:83`、`_is_verification_required:93`、`_is_rate_limited:109`、`_header_value:128`、`_response_navigation:140`、`_navigation_kind:159`、`_looks_like_html:168`、`_html_looks_like_login:173`、`_parse_cookies:189`、`_csrf:197`、`_normalize_server_identifier:200`、`_cookie_jar_dict:208`、`BuffBuyer:227`、`__init__:228`、`user_agent:292`、`export_cookie_string:297`、`close:305`、`_sync_session_cookies:312`、`_merge_response_cookies:317`、`reset_request_circuit:347`、`_make_request:354`、`verify_session:568`、`get_steam_trades:596`、`check_wait_pay_orders:616`、`get_sell_orders:654`、`get_goods_steam_price_cny:678`、`preview_buy:706`、`_preview_payment_error:743`、`get_and_buy:766`、`_execute_post_buy:815`、`_fetch_pay_url:900`、`lock_and_get_pay_url:935`、`_get_alipay_url:1065`、`_get_wechat_pay_url:1082`、`_fetch_wechat_url:1095`、`_generate_qr_code:1103`、`batch_buy_create:1120`、`batch_buy_wx_qrcode:1130`、`batch_buy_finalize:1146`、`ask_seller_to_send:1157`

依赖：`.request_policy`、`copy`、`json`、`logging`、`qrcode`、`requests`、`requests.cookies`、`time`、`typing`、`urllib.parse`、`urllib3`、`utils.delay`

### buff/request_policy.py

SHA-256：`1abc5ae04f7b9b09887143f134baef504e188c213be1fa3d78cb8e159bd3b019`

符号：`_retry_fallback:34`、`_reject_non_finite_json:44`、`_reject_duplicate_json_keys:48`、`BuffRequestError:57`、`BuffAuthExpired:61`、`BuffWriteResultUnknown:65`、`__init__:68`、`BuffRequestBlocked:82`、`BuffVerificationRequired:86`、`BuffRiskControlTriggered:90`、`__init__:93`、`BuffRateLimited:98`、`__init__:101`、`_CircuitState:108`、`from_json:116`、`account_fingerprint:166`、`parse_retry_after:188`、`BuffRequestPolicy:221`、`__init__:224`、`_account_lock:251`、`request_slot:256`、`raise_if_blocked:272`、`trip_verification:302`、`trip_risk_control:312`、`trip_rate_limit:325`、`_set_circuit:341`、`reset:354`、`clear:370`、`note_external_request:387`、`_load:398`、`_save_locked:452`、`get_global_policy:502`、`reset_global_policy:510`、`clear_global_policy:536`

依赖：`__future__`、`contextlib`、`dataclasses`、`datetime`、`email.utils`、`hashlib`、`json`、`logging`、`math`、`os`、`pathlib`、`tempfile`、`threading`、`time`、`typing`

### config/__init__.py

SHA-256：`e82619506692a0c5f42d5cda6815b5623c6ef621f5247686600c7278acf3ed4d`

符号：`_atomic_write_json:12`、`_load:32`、`get:47`、`get_steam:54`、`get_buff:57`、`get_all_credentials:60`、`save_credentials:63`、`_filter_steam_cookies:79`、`_steam_id_from_cookies:89`、`update_steam_credentials:101`、`update_buff_credentials:114`、`get_app_config_path:144`、`load_app_config:146`、`save_app_config:156`

依赖：`copy`、`json`、`os`、`pathlib`、`tempfile`、`threading`、`typing`

### run.py

SHA-256：`84f6b41d5cdc66d79548f21bbfe98591d8f1f4bc3d9f4304bf042c2d9041e4cd`

符号：包入口，无定义

依赖：`app.main`、`pathlib`、`sys`

### steam/__init__.py

SHA-256：`e2f2f2b1d58e8fab08f7ad6da3921f560ddc3997855f1a9a05f530221abfff8b`

符号：包入口，无定义

依赖：`.client`、`.inventory`、`.market`、`.session`

### steam/client.py

SHA-256：`d3525a9dda3472547ebcb9077af0b49468e16b7489f6f0e17ae915ca34db5ac0`

符号：`_coerce_listing_url:28`、`_parse_listing_url:36`、`_is_market_group_id:55`、`_extract_ssr_render_context:59`、`_extract_ssr_queries:69`、`_extract_market_query_name:81`、`_normalize_market_tag:93`、`_extract_category_filters:100`、`_description_matches_filters:112`、`_description_market_hash_name:123`、`_extract_group_market_hash_name:130`、`_extract_line1:155`、`detect_currency:173`、`build_listing_url:203`、`market_hash_name_from_listing_url:206`、`resolve_market_hash_name_from_listing_url:216`、`_parse_cookie_str:250`、`fetch_history:258`

依赖：`json`、`os`、`re`、`requests`、`typing`、`urllib.parse`、`urllib3`

### steam/inventory.py

SHA-256：`1ac1ac4ab614dc6f631af870736da9fe55e3a5fe578efa3683538aeead7d9eb8`

符号：`SteamAuthExpired:2`、`fetch_inventory:9`、`check_session_validity:30`、`_fetch_full_context:38`、`fetch_cs2_inventory:109`、`_fetch_ctx:119`、`find_asset_by_name:151`

依赖：`concurrent.futures`、`time`、`typing`、`utils.delay`

### steam/market.py

SHA-256：`34d40a75dc213615e7cde1257711b07059a4f12a8864858251260a8a6041ab9d`

符号：`parse_sell_response:6`、`list_item:27`、`list_item_by_name:49`

依赖：`.inventory`、`json`、`typing`

### steam/market_orders.py

SHA-256：`e12f51b198a4e3ee4f75495c9314b46ca79f1dc62bd42f031044a0fb288c0e79`

符号：`clear_caches:25`、`_format_request_error:107`、`_http_error_reason:113`、`_extract_item_nameid:122`、`_load_exchange_rates:129`、`_extract_ssr_render_context:152`、`_normalize_market_hash_name:162`、`_extract_orderbook_query_name:167`、`_extract_description_query_name:178`、`_extract_ssr_queries:189`、`_strip_html_text:202`、`_extract_description_tag:207`、`_extract_ssr_description_data:216`、`_infer_cs2_quality_filter_tag:236`、`_extract_exterior_name_from_description:246`、`_build_filtered_group_listing_url:261`、`_steam_cents_to_cny:293`、`_parse_compact_orders_cny:312`、`_compact_orderbook_data_to_cny:333`、`_extract_ssr_orderbook_cny:372`、`_fetch_ssr_sell_orders_cny:418`、`_fetch_action_orderbook_cny:475`、`get_item_nameid:548`、`fetch_item_orders_histogram:605`、`cents_to_yuan:694`、`_parse_sell_order_graph:696`、`get_sell_orders_cny:710`、`_get_dynamic_thresholds:809`、`compute_smart_list_price:819`、`get_lowest_sell_price_cny:866`

依赖：`app.database`、`json`、`logging`、`math`、`pathlib`、`re`、`steam.client`、`threading`、`time`、`typing`、`urllib.parse`、`utils.delay`、`utils.money`、`utils.proxy_manager`

### steam/session.py

SHA-256：`9f6a8f7723176049efcd72866aaea7dac2ffa181794b2d3a849c3ccdcdb34d7c`

符号：`parse_cookies:9`、`create_market_session:17`

依赖：`requests`、`typing`、`urllib3`

### steamdt/__init__.py

SHA-256：`13c5ea51b67e13f96604ad82918f418a1a85613ddef040679f9f1013e3269968`

符号：包入口，无定义

依赖：`.fetcher`、`.models`

### steamdt/fetcher.py

SHA-256：`752bc61c63903aacc2150a0bf7b30249a039450d0f6a116dcc5ef18d2cef86c4`

符号：`_make_timestamp:22`、`fetch_steamdt_data:30`

依赖：`.models`、`.parser`、`json`、`random`、`time`、`typing`、`urllib.error`、`urllib.request`

### steamdt/models.py

SHA-256：`f2bb73b1a648ba4fdb96621330d7e0a2c915556e1f9960f24a3ddc6c1ea70e2f`

符号：`SteamDTQueryParams:6`、`to_payload:20`、`SteamDTRow:36`

依赖：`dataclasses`、`typing`

### steamdt/parser.py

SHA-256：`1e94e4fc5bdb514dd6449e016c253909f9042d65aa0d22d2a51f2bae5e07f402`

符号：`_find_platform:6`、`parse_response:13`

依赖：`.models`、`typing`

### tests/__init__.py

SHA-256：`7571cd0be9938043ac5f16661613e49e35e8bee378b7c5e1cb3de5184e23a27e`

符号：包入口，无定义

依赖：无 import

### tests/test_account_region_sync.py

SHA-256：`acbb07b600a7708aeee36a969403fdb50c16f2bee1c0ca1a282a606159f1e2e5`

符号：`_ProxyManager:9`、`get_proxies_for_request:10`、`_Response:14`、`__init__:17`、`json:21`、`test_refresh_account_region_currency_success_marks_account_safe:25`、`test_refresh_account_region_currency_uses_settlement_currency_as_region_source:49`、`test_refresh_account_region_currency_can_fallback_to_wallet_country_for_unknown_currency:76`、`test_refresh_account_region_currency_marks_missing_currency_unsafe:102`、`test_refresh_account_region_currency_skips_missing_cookie_during_startup:123`、`test_refresh_account_region_currency_marks_missing_cookie_unsafe_when_strict:143`、`test_refresh_account_region_currency_rejects_cookie_for_different_account:162`、`test_verify_account_triggers_region_currency_refresh:194`、`test_get_base_auth_status_strict_reads_store_user_config_country:215`、`test_get_wallet_balance_returns_wallet_country:238`、`test_get_wallet_balance_fallback_returns_currency_for_zero_balance:257`、`test_get_wallet_balance_rejects_unknown_currency_id:281`、`test_gift_balance_rejects_cookie_for_different_current_account:301`、`test_gift_balance_syncs_wallet_currency_to_current_account:320`

依赖：`app`、`app.routes`、`app.services`、`html`、`json`、`pathlib`、`sys`、`utils.proxy_manager`

### tests/test_auth_relogin_state.py

SHA-256：`12efe095d8272890db81b907552b034692eeb32ea6389957a80e144bb5905320`

符号：`_isolated_global_buff_policy:9`、`test_manual_buff_cookie_requires_session:21`、`test_cookie_update_only_resumes_pure_auth_failures:44`、`test_relogin_finish_surfaces_worker_error:72`、`test_buff_auto_relogin_success_clears_auth_and_verification:90`、`FakePage:95`、`goto:98`、`wait_for_timeout:101`、`title:104`、`content:107`、`evaluate:110`、`FakeContext:123`、`cookies:126`、`close:133`、`FakeChromium:136`、`launch_persistent_context:137`、`FakePlaywright:140`、`__enter__:141`、`__exit__:144`、`test_relogin_context_retries_with_temp_profile:183`、`FakeChromium:188`、`launch_persistent_context:189`、`test_browser_launch_error_is_user_friendly:209`、`test_steam_auto_relogin_does_not_reuse_cookie_from_different_account:224`

依赖：`app`、`app.routes`、`app.services`、`buff`、`pytest`、`sys`、`threading`、`types`

### tests/test_background_startup.py

SHA-256：`b47e1b8c4a2bee8cc62b814faa35e366926a800693ee72ce13911096e53c7512`

符号：`test_task_queue_has_spare_capacity_for_startup_workers:7`、`test_region_sync_is_submitted_before_long_running_workers:16`、`FakeQueue:21`、`submit:22`、`test_wait_for_server_ready_probes_localhost_for_wildcard_host:35`、`FakeThread:40`、`is_alive:41`、`FakeSocket:44`、`__enter__:45`、`__exit__:48`、`fake_create_connection:51`、`test_wait_for_server_ready_stops_when_server_thread_exits:61`、`DeadThread:64`、`is_alive:65`、`fail_if_called:68`、`test_region_sync_worker_does_not_sleep_before_missing_account_skip:76`

依赖：`app`、`app.services`、`pathlib`、`sys`

### tests/test_buff_ask_seller.py

SHA-256：`fef6da3d6f19ff70e74c444c688b0541ae9b3ea32f0208ba8500c0d99123afee`

符号：`_buyer_with_responses:13`、`make_request:18`、`test_all_bill_orders_are_sent_once_and_all_ok_is_success:35`、`test_order_ids_are_normalized_and_deduplicated_in_one_request:62`、`test_partial_per_order_failure_is_not_complete_success:77`、`test_missing_per_order_status_is_not_complete_success:103`、`test_top_level_failure_is_not_retried:115`、`test_unknown_write_result_is_never_retried:128`

依赖：`buff`、`buff.buyer`、`json`、`pathlib`、`pytest`、`sys`

### tests/test_buff_auth_hardening.py

SHA-256：`0c08e936b805fdb4180769f4eab97700734169da4d8b9ec20d8665934153ebec`

符号：`_isolated_global_buff_policy:9`、`test_buff_credentials_keep_identity_metadata_and_advance_generation:21`、`test_manual_buff_cookie_saves_stable_user_agent_and_source:52`、`test_manual_buff_cookie_requires_csrf_token:80`、`test_failed_manual_cookie_probe_never_saves_clears_or_resumes:98`、`test_manual_probe_uses_one_global_paced_default_account_request:133`、`Buyer:142`、`__init__:143`、`verify_session:147`、`export_cookie_string:151`、`close:154`、`test_manual_probe_persists_new_rate_limit_to_global_policy:175`、`Buyer:182`、`__init__:183`、`verify_session:186`、`close:189`、`Policy:192`、`trip_rate_limit:193`、`test_same_manual_cookie_does_not_probe_an_open_circuit:207`、`BlockedPolicy:211`、`raise_if_blocked:212`、`test_changed_manual_cookie_cannot_bypass_verification_circuit:226`、`BlockedPolicy:230`、`raise_if_blocked:231`、`test_changed_cookie_still_cannot_bypass_active_rate_limit:246`、`RateLimitedPolicy:250`、`raise_if_blocked:251`、`test_verified_clear_targets_default_account_only:265`、`Policy:272`、`raise_if_blocked:273`、`clear:276`、`test_verified_clear_preserves_unexpired_rate_limit:286`、`Policy:290`、`raise_if_blocked:291`、`clear:294`、`test_verified_clear_preserves_global_pacing_timestamp:302`、`test_verified_clear_recovers_fail_closed_invalid_policy_state:319`、`test_browser_verification_script_has_hard_timeout_and_global_pacing:331`、`Page:339`、`title:342`、`content:345`、`evaluate:348`、`test_browser_verification_rejects_ok_without_user_info:372`、`Page:377`、`title:380`、`content:383`、`evaluate:386`、`test_browser_verification_trips_global_risk_circuit:401`、`Page:409`、`evaluate:412`、`test_browser_verification_429_uses_retry_after_and_trips_global_rate_limit:429`、`Page:436`、`evaluate:439`、`test_duplicate_relogin_start_is_rejected_without_closing_active_context:457`、`AliveThread:460`、`is_alive:461`、`Context:464`、`close:465`、`test_active_rate_limit_rejects_browser_relogin_before_thread_start:478`、`test_checkout_pending_rejects_manual_and_browser_credential_replacement:514`、`test_checkout_state_freezes_buff_credentials:566`、`test_abandoned_relogin_times_out_and_releases_buff_lock:587`、`Page:590`、`goto:591`、`Context:594`、`close:597`、`Playwright:600`、`stop:601`、`test_bulk_credentials_import_rebases_generation_and_is_atomic:632`、`test_auto_keepalive_never_opens_profile_for_manual_credentials:654`、`test_auto_keepalive_does_not_replace_credentials_during_checkout:669`、`test_auto_keepalive_does_not_probe_an_open_circuit:695`、`test_challenge_page_is_not_accepted_and_trips_global_verification:714`、`ChallengePage:721`、`title:724`、`content:727`、`evaluate:730`、`test_inactive_captcha_script_does_not_look_like_active_challenge:743`、`NormalPage:746`、`title:749`、`content:752`、`test_session_keepalive_requires_idle_non_checkout_pipeline:758`、`test_buff_session_keepalive_is_opt_in_by_default:778`、`test_keepalive_first_wait_uses_full_configured_interval:786`、`stop_on_sleep:799`

依赖：`app.config_schema`、`app.routes`、`app.services`、`app.services.buff_auth`、`app.services.workers`、`buff`、`buff.request_policy`、`config`、`json`、`pytest`、`threading`、`types`

### tests/test_buff_batch_integrity.py

SHA-256：`be65243ff9ea13f292d5246c238bee012f61887dbfb9fae387aff450b107e935`

符号：`test_legacy_batch_writes_are_blocked_before_http:22`、`make_request:27`、`test_client_batch_buy_is_safe_not_supported_fallback_without_running_buyer:39`、`test_exact_budget_multiple_does_not_plan_one_item_too_few:61`、`_client_using:68`、`test_finalize_skips_duplicate_sell_rows_without_sending_duplicate_post:74`、`Buyer:75`、`__init__:76`、`get_sell_orders:79`、`batch_buy_finalize:86`、`test_duplicate_bill_id_halts_with_only_prior_unique_results:111`、`Buyer:112`、`__init__:113`、`get_sell_orders:116`、`batch_buy_finalize:123`、`test_pipeline_rejects_duplicate_ids_before_persisting_complete_batch:155`

依赖：`app.pipeline_steps`、`app.services.buff_client`、`buff`、`buff.buyer`、`pytest`

### tests/test_buff_checkout_guard.py

SHA-256：`80f65a4e7ee2725b9db7b610219b3b670c8df8bc5039c3cb159ecbf44fa038e1`

符号：`_isolated_guard:9`、`test_intent_is_durable_secret_free_and_blocks_a_second_checkout:15`、`test_update_then_resolve_allows_a_new_intent:38`、`test_acknowledgement_is_explicit_and_persists:58`、`test_corrupt_journal_fails_closed:71`、`test_duplicate_journal_keys_fail_closed:82`、`test_non_finite_journal_numbers_fail_closed:95`、`test_parseable_but_invalid_journal_fails_closed:133`、`test_stale_acknowledgement_cannot_clear_a_new_intent:147`

依赖：`app.services`、`json`、`pytest`

### tests/test_buff_checkout_integration.py

SHA-256：`09d7fe24f1af705a1ba172d0fa915550be2428d152d1ceb35c0d841b14b1eee3`

符号：`_State:10`、`__init__:11`、`get_status:23`、`set_status:26`、`set_pending_payment:29`、`log:32`、`_isolated_guard:37`、`test_unresolved_checkout_blocks_restart_until_explicit_ack:54`、`test_restart_ack_requires_the_exact_displayed_intent_id:85`、`test_pipeline_start_does_not_clear_unverified_auth_flags:115`、`test_durable_guard_freezes_credentials_and_background_reads:131`、`test_running_pipeline_freezes_manual_credential_replacement:152`、`test_full_import_rejects_during_atomic_checkout_then_succeeds:165`、`BuffClient:182`、`get_credential_identity:185`、`lock_and_get_pay_url:194`、`run_checkout:247`、`run_import:266`

依赖：`app`、`app.routes`、`app.services`、`pytest`、`threading`、`time`

### tests/test_buff_purchase_flow_safety.py

SHA-256：`ad585b5496f1f8f1bf0c7a21432873fa06c34a9d794e3fb48be086479a2a6591`

符号：`_isolated_checkout_guard:12`、`_config:22`、`_item:34`、`_checkout_args:47`、`_FakeState:61`、`__init__:62`、`clear_stop:69`、`is_stop_requested:72`、`set_buff_auth_expired:75`、`set_buff_verification_required:78`、`set_status:81`、`log:84`、`set_pending_payment:87`、`wait_payment_confirm:90`、`confirm_payment:93`、`append_purchase:96`、`_FakeContext:100`、`__init__:101`、`is_stop_requested:105`、`log:108`、`set_status:111`、`debug:114`、`test_realtime_buff_check_runs_only_after_other_candidate_guards:118`、`BuffClient:125`、`get_sell_orders:126`、`test_closed_time_window_blocks_checkout_post_after_validation:161`、`BuffClient:162`、`__init__:165`、`lock_and_get_pay_url:168`、`test_time_window_closing_during_candidate_analysis_skips_checkout:191`、`test_time_window_closing_after_first_purchase_prevents_second:230`、`checkout:243`、`test_stale_buff_cache_is_refreshed_before_locking:274`、`BuffClient:275`、`__init__:278`、`get_sell_orders:282`、`lock_and_get_pay_url:286`、`test_single_order_payment_cancel_does_not_lock_again:310`、`BuffClient:311`、`__init__:314`、`get_sell_orders:317`、`lock_and_get_pay_url:320`、`test_rate_limit_is_not_swallowed_or_retried:346`、`BuffClient:347`、`__init__:350`、`get_sell_orders:353`、`lock_and_get_pay_url:356`、`test_failed_checkout_session_preflight_sends_no_post_or_guard:374`、`BuffClient:375`、`__init__:378`、`verify_session:382`、`lock_and_get_pay_url:386`、`test_failed_read_only_buy_preview_sends_no_post_or_guard:408`、`BuffClient:409`、`__init__:412`、`verify_session:416`、`prepare_single_buy:419`、`lock_and_get_pay_url:428`、`test_successful_preview_is_reused_by_the_immediate_checkout_post:450`、`BuffClient:453`、`__init__:456`、`verify_session:459`、`prepare_single_buy:462`、`lock_and_get_pay_url:465`、`test_definitive_login_rejection_resolves_guard_and_sends_one_post:483`、`BuffClient:484`、`__init__:487`、`verify_session:491`、`lock_and_get_pay_url:495`、`test_transport_unknown_write_is_converted_to_terminal_purchase_state:524`、`BuffClient:525`、`__init__:528`、`lock_and_get_pay_url:531`、`test_batch_payment_cancel_does_not_fallback_to_single_order:553`、`BuffClient:554`、`__init__:557`、`get_sell_orders:562`、`try_batch_buy:565`、`lock_and_get_pay_url:574`、`batch_buy_find_and_finalize:578`、`test_unknown_batch_result_does_not_fallback_to_single_order:603`、`BuffClient:604`、`__init__:607`、`get_sell_orders:610`、`try_batch_buy:613`、`lock_and_get_pay_url:616`、`test_explicitly_unsupported_batch_can_fallback_once:637`、`BuffClient:638`、`__init__:641`、`get_sell_orders:645`、`try_batch_buy:648`、`lock_and_get_pay_url:652`、`test_client_without_batch_support_falls_back_to_one_single_post:674`、`BuffClient:675`、`__init__:679`、`verify_session:683`、`try_batch_buy:686`、`lock_and_get_pay_url:690`、`test_trusted_not_created_batch_response_can_fallback_once:714`、`BuffClient:715`、`__init__:718`、`try_batch_buy:722`、`lock_and_get_pay_url:731`、`test_unknown_write_result_stops_entire_pipeline_with_explicit_status:753`、`BuffClient:757`、`__init__:760`、`lock_and_get_pay_url:763`、`ProxyManager:771`、`is_proxy_enabled:772`、`NetworkChecker:775`、`report_success:776`、`test_purchase_amount_is_counted_when_post_commit_shipping_prompt_is_blocked:821`、`BuffClient:826`、`lock_and_get_pay_url:829`、`ask_seller_to_send:837`、`test_successful_batch_records_reconciliation_identifiers:871`、`BuffClient:872`、`try_batch_buy:875`、`batch_buy_find_and_finalize:883`、`ask_seller_to_send:889`、`test_partial_batch_finalize_records_committed_items_and_halts:911`、`BuffClient:912`、`__init__:915`、`try_batch_buy:918`、`batch_buy_find_and_finalize:926`、`ask_seller_to_send:931`、`test_batch_finalize_unknown_persists_prior_successes_before_halting:956`、`BuffClient:957`、`try_batch_buy:960`、`batch_buy_find_and_finalize:968`、`test_batch_created_without_pay_url_is_terminal_and_never_falls_back:1005`、`BuffClient:1006`、`__init__:1009`、`try_batch_buy:1012`、`lock_and_get_pay_url:1020`、`test_single_created_without_pay_url_keeps_order_id_and_never_auto_resumes:1042`、`BuffClient:1043`、`lock_and_get_pay_url:1046`、`test_cooling_down_has_dedicated_terminal_state:1069`、`BuffClient:1070`、`lock_and_get_pay_url:1073`、`test_expired_snapshot_immediately_before_write_sends_no_post:1087`、`BuffClient:1088`、`__init__:1091`、`lock_and_get_pay_url:1094`、`test_invalid_zero_lowest_price_is_rejected_before_quantity_math:1118`、`BuffClient:1119`、`lock_and_get_pay_url:1122`、`test_pending_payment_is_cleared_when_wait_callback_raises:1137`、`fail_wait:1140`、`test_unrecognised_non_ok_write_result_stays_unresolved:1163`、`BuffClient:1164`、`lock_and_get_pay_url:1167`、`test_single_order_id_is_durable_before_payment_link_fetch_can_crash:1185`、`BuffClient:1186`、`lock_and_get_pay_url:1189`、`test_batch_id_is_durable_before_qr_fetch_can_crash:1214`、`BuffClient:1215`、`try_batch_buy:1218`、`test_each_batch_bill_id_is_durable_before_the_next_finalize_post:1244`、`BuffClient:1245`、`try_batch_buy:1248`、`batch_buy_find_and_finalize:1256`、`test_all_batch_ids_are_durable_before_local_db_append_failure:1289`、`BuffClient:1290`、`try_batch_buy:1293`、`batch_buy_find_and_finalize:1301`、`fail_second_append:1310`、`test_automatic_shipping_prompt_is_disabled_by_default:1342`、`BuffClient:1343`、`__init__:1346`、`lock_and_get_pay_url:1349`、`ask_seller_to_send:1357`、`test_opted_in_shipping_prompt_unknown_halts_after_recording:1381`、`BuffClient:1382`、`lock_and_get_pay_url:1385`、`ask_seller_to_send:1393`、`test_batch_shipping_prompt_unknown_preserves_all_committed_items:1424`、`BuffClient:1425`、`try_batch_buy:1428`、`batch_buy_find_and_finalize:1436`、`ask_seller_to_send:1442`、`test_guarded_runner_sets_error_for_unhandled_exception:1481`

依赖：`app`、`app.services`、`app.services.buff_checkout_guard`、`buff`、`buff.request_policy`、`pytest`、`time`

### tests/test_buff_purchase_persistence.py

SHA-256：`1995859f18cb089e21a12a90d49506e5000d769fe49bc91785fb9f613711ba46`

符号：`test_purchase_round_trip_keeps_buff_reconciliation_ids:1`

依赖：`app`

### tests/test_buff_receive_mapping_safety.py

SHA-256：`c1b1c8d5c472688bb43fc8ecb1ea004a1decc3ffe81f015fb1374ed90238c80f`

符号：`test_accept_timeout_is_unknown_and_non_idempotent_post_is_not_retried:9`、`ProxyManager:14`、`get_proxies_for_request:15`、`timeout_once:18`、`test_exact_goods_id_match_beats_older_same_name_fallback:37`、`test_ambiguous_name_only_match_is_rejected:63`、`test_partial_inventory_visibility_never_persists_seller_assetid:75`、`get_purchases:112`、`update_purchase_by_id:115`、`accept_offer:120`、`scan_inventory:124`、`test_inventory_poll_waits_for_all_new_assets_and_excludes_old_same_name:178`、`get_purchases:216`、`update_purchase_by_id:219`、`accept_offer:224`、`inventory_item:228`、`scan_inventory:231`、`test_failed_pre_accept_inventory_snapshot_leaves_offer_unaccepted:272`、`test_unmatched_offer_is_never_accepted_or_scanned:325`、`test_unknown_accept_result_is_reconciled_from_inventory:376`、`get_purchases:399`、`update_purchase_by_id:402`、`accept_unknown:407`、`scan_inventory:411`

依赖：`app`、`requests`

### tests/test_buff_receive_session.py

SHA-256：`bb72368de8a60869d03e2d1b4cd67b7a052233274fda7b797fbb818da5312347`

符号：`test_buff_trade_poll_uses_shared_client_and_parses_pending_items:4`、`unexpected_request:7`、`Client:12`、`__init__:13`、`get_steam_trades:16`、`test_buff_trade_poll_propagates_request_circuit:55`、`Client:59`、`get_steam_trades:60`、`test_buff_background_poll_only_runs_while_fully_idle:80`

依赖：`app.receive_flow`、`app.services`、`buff`、`pytest`

### tests/test_buff_request_policy.py

SHA-256：`24e38a251f242527acfa1c109fd89343a17a74f3bdf7f944f7804736c56746fc`

符号：`FakeResponse:36`、`__init__:37`、`json:62`、`FakeSession:68`、`__init__:69`、`request:76`、`close:85`、`no_wait_policy:89`、`user_info_response:96`、`buy_preview_response:110`、`checkout_responses:126`、`test_default_user_agent_is_stable_and_can_be_overridden:136`、`test_close_only_releases_an_internally_owned_session:149`、`test_persistent_session_accepts_cookie_and_syncs_csrf_before_write:165`、`test_429_retry_after_opens_and_persists_account_circuit:188`、`test_damaged_policy_state_fails_closed_until_explicit_clear:257`、`test_rate_limit_record_without_deadline_is_invalid_and_blocks:271`、`test_failed_policy_replace_leaves_restart_fail_closed_marker:297`、`test_risk_http_status_opens_persistent_circuit_until_explicit_reset:339`、`test_get_sell_orders_non_ok_does_not_retry:375`、`test_legacy_get_and_buy_never_falls_through_to_second_sell_order:385`、`test_configured_request_timeout_is_used_by_default:420`、`test_policy_enforces_hard_minimum_interval_for_same_account:434`、`clock:438`、`sleep:441`、`test_production_defaults_use_two_second_interval_and_five_minute_429:456`、`test_invalid_numeric_retry_after_uses_safe_default:475`、`test_non_finite_or_negative_retry_defaults_are_also_rejected:479`、`test_cooling_down_response_opens_default_rate_limit_circuit:488`、`test_owned_session_ignores_environment_proxy_settings:505`、`test_buff_client_uses_stable_default_account_key_across_session_rotation:515`、`test_buff_client_reloads_changed_credentials_without_generation_bump:535`、`test_missing_csrf_blocks_write_before_http_call:562`、`test_waiting_writer_reads_csrf_only_after_preceding_response_rotates_it:573`、`ConcurrentSession:577`、`request:578`、`run:597`、`test_write_network_timeout_is_explicit_unknown_result:617`、`test_public_lock_method_does_not_swallow_unknown_write_result:631`、`test_success_without_order_id_is_unknown_and_skips_payment_lookup:648`、`test_legacy_batch_writes_are_locally_blocked_without_http:675`、`test_every_order_creating_write_rejects_malformed_success_data:700`、`test_execute_preflights_user_info_and_clickable_payment_method_before_post:724`、`test_known_steam_id_skips_user_info_but_never_skips_buy_preview:754`、`test_single_checkout_never_posts_without_explicit_clickable_payment_method:789`、`test_exact_http_200_login_required_after_post_is_known_auth_rejection:815`、`test_buy_reuses_server_cashier_trace_and_current_browser_write_headers:841`、`test_buy_never_invents_cashier_trace_when_server_did_not_issue_one:873`、`test_non_idempotent_5xx_or_non_json_is_unknown_result:900`、`test_every_issued_abnormal_write_is_unknown_and_keeps_relevant_circuit:1015`、`test_order_created_payment_lookup_failure_returns_created_pending:1045`、`test_payment_lookup_block_is_preserved_after_created_pending_result:1077`、`test_wechat_payment_lookup_auth_failure_is_created_pending:1101`、`test_created_order_with_valid_payment_url_keeps_success_contract:1131`、`test_login_redirect_is_auth_expired_and_verification_redirect_is_fused:1160`、`test_unknown_html_read_fails_closed:1203`、`test_initial_cookie_is_domain_scoped_and_expiration_removes_it:1218`、`test_verify_session_is_one_lightweight_get_and_trade_poll_uses_same_session:1246`、`test_verify_session_rejects_responses_without_a_real_user:1284`、`test_waiting_payment_history_uses_current_state_contract_without_state_param:1297`、`test_cookie_persistence_failure_never_masks_operation_result_or_exception:1345`、`persistence_failure:1348`、`Buyer:1351`、`__init__:1354`、`get_sell_orders:1357`、`export_cookie_string:1362`、`close:1365`

依赖：`app.services.buff_client`、`buff.buyer`、`buff.request_policy`、`json`、`pytest`、`requests`、`requests.cookies`、`threading`

### tests/test_buff_verification.py

SHA-256：`48b0483f107a02d7602b75f2d0b713cebdbbbd32c164df718f35ce5b0b566a2f`

符号：`test_buff_page_expired_is_verification_required:4`、`test_make_request_raises_verification_required:10`、`FakeResponse:13`、`json:17`

依赖：`buff.buyer`、`pytest`

### tests/test_config_loader_validation.py

SHA-256：`65b4f0f7a88623ba11647981e29a3ed246ffaaa57231c32dcc487d395470c513`

符号：`test_validate_and_fill_preserves_real_boolean_values:10`、`test_load_app_config_validated_applies_range_validation:45`、`test_save_app_config_validated_applies_range_validation:59`、`test_update_app_config_validated_merges_partial_sections:72`、`fake_save:89`、`test_concurrent_partial_config_updates_do_not_overwrite_each_other:109`、`fake_save:130`、`update:146`、`test_config_cache_reloads_external_file_changes_and_returns_copies:189`、`test_config_cache_retries_when_file_changes_during_read:222`、`fake_revision:231`、`fake_load:234`

依赖：`app`、`app.config_schema`、`config`、`copy`、`pathlib`、`sys`、`threading`、`warnings`

### tests/test_config_routes.py

SHA-256：`17b16251e627d5e0107a7ede284b2befb470dcdcad8bf8213268f60bc18a5456`

符号：`test_config_route_persists_all_settings_checkboxes:8`、`fake_update:19`、`test_config_get_disables_http_caching:50`、`test_full_import_uses_validated_config_save:61`、`test_full_credentials_import_holds_buff_auth_lock:88`、`Lock:94`、`acquire:95`、`release:99`、`test_full_import_explicit_empty_sections_clear_only_those_sections:119`

依赖：`app.config_schema`、`app.routes`、`app.services`、`copy`、`fastapi`、`pathlib`、`sys`

### tests/test_config_schema.py

SHA-256：`d0eb860523995683786e7013d35ce74e78c88e482122bd5fe5b563502487e069`

符号：`test_merge_顶层覆盖:12`、`test_merge_深层合并不影响未指定key:18`、`test_merge_非dict会整体覆盖:26`、`test_merge_不修改原始defaults:31`、`test_validate_字符串转float:39`、`test_validate_字符串转int:45`、`test_validate_iflow_min_price_zero_is_preserved:50`、`test_validate_缺少section用默认值:55`、`test_range_max_discount超出被限制:62`、`test_range_cv_threshold为零被限制:71`、`test_range_正常值不被修改:79`、`test_range_price_tolerance负数被限制:90`、`test_range_inventory_refresh_seconds_too_short_is_lifted_to_ten_minutes:99`

依赖：`app.config_schema`、`pathlib`、`sys`、`warnings`

### tests/test_config_schema_hardening.py

SHA-256：`a741d99b85c4bbadfbf4fee59605e443ba951b99d2796701716e48a9e7c23733`

符号：`test_merge_deepcopies_nested_mutable_defaults:9`、`test_validate_bool_strings_use_boolean_semantics:16`

依赖：`app.config_schema`、`pathlib`、`sys`

### tests/test_currency_conversion.py

SHA-256：`8492fa60416c8247c81efaebbba1f81b899f59df78c184d46c4d06e8fe63a520`

符号：`test_detect_currency_supports_steam_symbol_prefixes:9`、`test_apply_currency_uses_rate_map_for_non_usd_prices:24`、`test_stability_currency_conversion_uses_exchange_rate_map:33`

依赖：`analysis`、`pathlib`、`pytest`、`steam.client`、`sys`、`utils.money`

### tests/test_database.py

SHA-256：`bd4b87d8b6d1ed39f12d56c28af9835c3d76a5752a8a5c8211b091bb8bb86f17`

符号：`test_wilson_score_无评价返回零:12`、`test_wilson_score_基本正确性:18`、`test_wilson_score_少量样本被惩罚:28`、`test_wilson_score_结果在01之间:35`

依赖：`app.database`、`pathlib`、`sys`

### tests/test_holdings_stats.py

SHA-256：`a70077fd5dbdcce157b61b66995176484d9ec7a1d61bbc8ab93b7d8b901eb522`

符号：`test_total_market_change_only_counts_items_with_both_market_prices:4`

依赖：`app.notify`

### tests/test_market_orders.py

SHA-256：`0aef70b600b9278bbabdab42c029d41a5f420c3aa6f2f8d53bd3e7b204698dfe`

符号：`test_新版订单簿支持INR转CNY:7`、`test_新版订单簿缺少非CNY汇率会报清晰原因:28`、`test_汇率文件里的Steam市场币种都有ECurrency映射:48`、`test_汇率文件包含的非Steam本地市场币种不会误映射:60`、`test_新版订单簿按querykey精确匹配请求的market_hash_name:69`、`test_get_sell_orders_cny_优先使用新版ssr而不是旧item_nameid缓存:115`、`test_get_sell_orders_cny_聚合页会回退到带筛选的分组页面:136`、`_make_html:141`、`DummyResponse:191`、`__init__:192`、`DummySession:196`、`get:197`、`test_构造分组筛选页优先使用描述里的内部tag:218`、`test_构造分组筛选页能识别星号stattrak品质:261`、`test_新版_orderbook_action_按精确名称拉取目标变体:268`、`DummyResponse:276`、`json:279`、`DummySession:289`、`get:290`、`test_新版_orderbook_action_兼容嵌套data响应:312`、`DummyResponse:315`、`json:318`、`DummySession:330`、`get:331`、`test_get_sell_orders_cny_ssr预取错变体时回退新版_orderbook_action:348`

依赖：`json`、`pathlib`、`steam`、`sys`

### tests/test_pipeline_lock.py

SHA-256：`915f2679359bccb0c169dbdd516e9a2e9311058daae40cdc75ea26109c9f48b5`

符号：`test_start_pipeline_rejects_duplicate_running_pipeline:4`、`fake_run_pipeline:10`、`test_start_pipeline_allows_restart_after_thread_exits:33`、`fake_run_pipeline:38`

依赖：`app`、`threading`

### tests/test_pipeline_steps.py

SHA-256：`216d625e3bd7dcb46d108e1eca0937f40f36d2681bc26390b545d2f24eb597d8`

符号：`_行:17`、`test_卖压_空订单返回None:42`、`test_卖压_日销量为零返回None:46`、`test_卖压_大量挂单卖压高:51`、`test_卖压_少量挂单卖压低:59`、`test_卖压_价格断层识别:66`、`test_过滤_正常行通过:79`、`test_过滤_关键词命中被去掉:85`、`test_过滤_价格非正被去掉:96`、`test_过滤_非buff链接被去掉:107`、`test_过滤_topN限制数量:117`、`test_过滤_goods_id从url解析:124`、`_steam_history_rows:130`、`test_daily_high_adjustment_uses_trimmed_average:142`、`DummySteamClient:145`、`fetch_history:146`、`test_daily_high_adjustment_never_raises_reference_price:162`、`DummySteamClient:165`、`fetch_history:166`

依赖：`app.pipeline_steps`、`datetime`、`pathlib`、`pytest`、`sys`、`types`

### tests/test_pipeline_time_window.py

SHA-256：`7106c8f78c4b0eafc4f21154c3e90985df0f9605f512d4b6c65fb6215782c2dd`

符号：`test_browser_timezone_converts_utc_container_clock_for_schedule:7`、`test_offset_fallback_works_when_iana_timezone_is_invalid:22`、`test_daytime_window_is_start_inclusive_and_end_exclusive:37`、`test_overnight_and_equal_windows_have_explicit_semantics:52`、`test_invalid_offset_falls_back_to_server_local_clock:67`、`test_legacy_config_defaults_to_china_business_timezone:79`、`test_runtime_config_coerces_string_boolean_and_invalid_hours:86`

依赖：`app`、`app.config_schema`、`datetime`

### tests/test_proxy_manager.py

SHA-256：`1d3f85a4f23000b8974ebb8d9aa4ee6b3c8dd5852d56b7503f9536c4c32c6154`

符号：`_构造代理管理器:10`、`test_代理_score为0的永远不被选:27`、`test_代理_空池返回None:39`、`test_代理_全挂了返回None:44`、`test_代理_高分代理被选得明显更多:54`、`test_代理池关闭时不预热:68`、`test_新配置代理在异步预热完成前可用:81`

依赖：`collections`、`pathlib`、`sys`、`threading`、`unittest.mock`、`utils`、`utils.proxy_manager`

### tests/test_repair_error_records.py

SHA-256：`56838bb38569d8a2b9ca4f40884a4b9c250b141d9b71920cad8544286e871758`

符号：`_prepare_repair:9`、`test_rebuild_preserves_sold_record_when_history_misses_it_even_if_inventory_has_same_name:55`、`test_rebuild_marks_original_assetid_sold_when_assetid_is_in_sold_history:96`、`test_rebuild_uses_exact_name_matching_for_missing_assetid:125`、`test_rebuild_clears_stale_unsold_assetid_when_not_seen_anywhere:149`、`test_rebuild_keeps_pending_receipt_when_not_seen_anywhere:173`、`test_fetch_sold_with_names_paginates_history:192`、`FakeResponse:197`、`__init__:201`、`json:204`、`row:207`、`fake_get:231`

依赖：`app`、`copy`、`json`、`pathlib`、`sys`

### tests/test_runtime_env.py

SHA-256：`5f93c0d08c716e7e8745f018f1b2bc198f737f4cdb7761ba4f746ce0a1542a29`

符号：`test_server_requirements_omit_desktop_webview_dependency:12`、`test_legacy_app_main_exposes_fastapi_app_lazily:19`、`test_auto_linux_without_display_uses_server_mode:25`、`test_no_display_stays_effectively_headless_even_if_env_disables_headless:34`、`test_auto_linux_with_display_uses_desktop_mode:42`、`test_env_overrides_host_port_and_local_url:50`、`test_relogin_start_requires_manual_cookie_without_gui:60`、`fail_thread:76`

依赖：`app.main`、`app.routes`、`app.runtime_env`、`pathlib`

### tests/test_sell_listing_response.py

SHA-256：`eae0eade44e737a32b5b89a2276db12aca248de21fa3db25859cd76d0d206fc2`

符号：`_entry:6`、`_context:21`、`_messages:27`、`test_submit_listing_handles_invalid_response_shapes:45`、`test_submit_listing_preserves_success_variants:86`、`test_submit_listing_retry_null_is_diagnostic_and_not_success:119`、`test_submit_listing_retry_success_is_recorded_once:168`、`test_submit_listing_non_string_message_is_safe:210`、`test_submit_listing_rejects_untrusted_success_shapes:250`、`test_listing_response_logs_are_single_line_bounded_and_redacted:279`、`test_parse_sell_response_contract:333`、`test_list_item_by_name_preserves_safe_response_contract:347`

依赖：`app`、`pytest`、`steam`、`unittest.mock`

### tests/test_sell_pipeline_currency_guard.py

SHA-256：`5e827d89ed93f970c198f82461d6995838304a636ad521a660289adb31ca0565`

符号：`_make_sellable:13`、`_build_ctx:24`、`_log:30`、`test_missing_rate_aborts_listing_plan:39`、`test_valid_rate_converts_price:75`、`test_no_account_skips_sell_phase:118`、`test_missing_currency_code_aborts_sell_phase:141`、`_log:147`、`test_sell_phase_uses_realtime_currency_instead_of_stale_cache:169`、`_log:175`、`test_sell_phase_does_not_require_region_when_currency_is_confirmed:199`

依赖：`app.sell_pipeline`、`pytest`、`unittest.mock`、`utils.money`

### tests/test_sell_pipeline_ownership_guard.py

SHA-256：`3c6928a73337a92965555cad67f215618278f58305da586496db9484f9f30f3b`

符号：`_purchase:9`、`_inventory_item:22`、`_ctx:33`、`test_historical_sold_same_name_does_not_authorize_different_asset:39`、`test_unsold_same_name_does_not_authorize_different_asset:49`、`test_exact_asset_must_still_be_a_current_holding:55`、`test_exact_current_asset_is_authorized:63`、`test_missing_inventory_assetid_fails_closed:69`、`test_listing_plan_never_prices_or_lists_same_name_personal_item:73`

依赖：`app.sell_pipeline`、`unittest.mock`

### tests/test_static_routes.py

SHA-256：`53efef42278f84e0f8bf503ea8f83e82c4c1b9bb66ec8ecc0af036dd7b902bfb`

符号：`test_static_route_serves_files_inside_web_root:7`、`test_static_route_blocks_path_traversal:22`、`test_versioned_settings_assets_and_uncached_api_fetches:40`

依赖：`app.routes`、`pathlib`、`sys`

### tests/test_status_time_export.py

SHA-256：`a426769e92c221e4faac2bd1e270746994a1f5d966727aa430915f28e321882d`

符号：`test_log_export_uses_configured_browser_timezone:1`

依赖：`app.routes`

### tests/test_steam_auth_errors.py

SHA-256：`13ca62254c38285303e0ea15217cd0f953a6f3d7ad759c80a45ea0afbc884a97`

符号：`_JsonResponse:10`、`__init__:13`、`json:17`、`test_steam_login_connection_error_message_is_explicit:21`、`test_nonstandard_http_443_is_classified_as_proxy_network_error:39`、`test_steam_login_error_redacts_proxy_credentials:51`、`test_steam_refresh_token_poll_retries_pending_response_and_tracks_new_client_id:65`、`FakeExecutor:77`、`_api_call:80`、`test_steam_refresh_token_poll_timeout_has_actionable_error:106`、`FakeExecutor:109`、`_api_call:112`、`test_steam_refresh_token_poll_timeout_mentions_mobile_confirmation:131`、`FakeExecutor:134`、`_api_call:137`、`test_do_steampy_login_repairs_upstream_single_poll_and_restores_patch:153`、`FakeSteamClient:162`、`__init__:163`、`login:176`、`fake_api_call:179`、`is_session_alive:190`、`test_do_steampy_login_once_injects_proxy_and_bounded_timeout:209`、`fake_request:216`、`FakeSteamClient:220`、`__init__:221`、`login:229`、`is_session_alive:232`、`test_steam_login_strategy_one_retries_network_error_through_proxy:257`、`FakeProxyManager:267`、`get_proxies_for_request:268`、`fake_login_once:272`、`test_steam_login_does_not_retry_authentication_error:297`、`FakeProxyManager:303`、`get_proxies_for_request:304`、`fake_login_once:308`、`test_steam_login_strategy_two_uses_proxy_and_rotates_once:331`、`FakeProxyManager:341`、`get_proxies_for_request:342`、`fake_login_once:346`、`test_steam_login_strategy_two_never_falls_back_to_direct:371`、`EmptyProxyManager:374`、`get_proxies_for_request:375`、`test_do_steampy_login_converts_refresh_token_key_error_to_auth_pending:403`、`FakeSteamClient:412`、`__init__:413`、`login:416`、`test_do_steampy_login_does_not_classify_pending_client_id_as_wrong_creds:437`、`FakeSteamClient:442`、`__init__:443`、`login:446`、`test_do_steampy_login_keeps_explicit_credentials_error_as_wrong_creds:463`、`FakeSteamClient:468`、`__init__:469`、`login:472`、`test_do_steampy_login_reads_explicit_credentials_error_from_initial_response:490`、`FakeSteamClient:498`、`__init__:499`、`login:502`、`test_do_steampy_login_releases_lock_when_login_executor_import_fails:528`、`test_verify_account_surfaces_auth_pending_instead_of_refresh_token:552`

依赖：`app.services`、`pathlib`、`pytest`、`requests`、`steampy`、`steampy.login`、`sys`、`types`

### tests/test_steam_client.py

SHA-256：`58f81b99cc4c64cd9b2974823ff3ae9c02c912d968d28dffb17aa17d2d74d0b9`

符号：`_ssr_html:13`、`_description:21`、`_orderbook:34`、`DummyResponse:41`、`__init__:44`、`DummySession:48`、`__init__:49`、`get:53`、`test_market_hash_name_from_old_listing_url:58`、`test_market_hash_name_from_group_listing_url_is_not_used_as_name:64`、`test_resolve_group_listing_url_uses_prefetched_orderbook_name:70`、`test_resolve_group_listing_url_honors_category_filters:89`

依赖：`json`、`pathlib`、`steam.client`、`sys`

### tests/test_steamdt_proxy.py

SHA-256：`09082ac57110b66e415587dd4a5b57426b6c5d4215bd0d4f8446b8e0aff2a4a6`

符号：`_response:8`、`Response:9`、`__enter__:12`、`__exit__:15`、`read:18`、`test_steamdt_proxy_provider_is_used_and_direct_mode_bypasses_environment:24`、`Opener:28`、`open:29`、`fake_build_opener:33`、`test_steamdt_network_error_retries_and_marks_failed_provider_call:61`、`Opener:65`、`open:66`、`test_iflow_client_wires_proxy_manager_into_steamdt:84`、`ProxyManager:89`、`get_proxies_for_request:90`、`fake_fetch:98`

依赖：`app.services.iflow_client`、`json`、`steamdt`、`types`、`urllib.error`

### tests/test_strategies.py

SHA-256：`af0ff1416c239d260b857c4e63688df4f3e10fc4f956a1710266b811a705cf2d`

符号：`test_legacy_sell_strategy_maps_to_system_presets:4`、`test_default_buy_strategy_contains_current_protection_chain:12`、`test_default_sell_strategy_uses_split_pricing_modules:36`、`test_activation_rejects_enabled_user_module:45`、`test_import_sanitizes_sensitive_fields_and_does_not_override_system_id:69`、`test_simulate_marks_trade_actions_as_dry_run:85`、`test_validation_rejects_duplicate_conflict_dependency_and_step_limit:105`、`test_split_sell_pricing_without_offset_resets_offset:151`、`test_sell_listing_requires_one_pricing_core:179`、`test_one_piece_sell_pricing_core_can_replace_split_pricing:195`、`test_declarative_user_module_can_activate_and_simulate:224`、`test_declarative_user_module_can_depend_on_one_piece_pricing:260`、`test_declarative_runtime_guard_blocks_at_stage:292`、`test_declarative_user_module_dependency_order_is_enforced:336`、`test_external_code_user_module_still_cannot_activate:364`、`test_step_param_validation_rejects_out_of_range_value:391`、`test_system_strategy_params_can_be_saved_to_config:408`、`fake_load:415`、`fake_save:418`、`test_system_strategy_params_can_restore_schema_defaults:437`、`fake_load:444`、`fake_save:447`、`test_system_strategy_save_rejects_structure_changes:470`

依赖：`app`

### tests/test_strategy_hardening.py

SHA-256：`711707c7fa438377d8ccf69a3253b38a1a4a1947f0d520cd35f8fe01d52f6baa`

符号：`test_strategy_import_enabled_string_false_stays_disabled:9`

依赖：`app`、`pathlib`、`sys`

### tests/test_time_utils.py

SHA-256：`00b5439d92e1c378d318709e5ed1916df353eaec4b01e3e9dd0a169f36c2850c`

符号：`test_steam_history_parser_preserves_instant_and_returns_utc:16`、`test_steam_history_utc_variants_are_timezone_aware:33`、`test_invalid_steam_history_dates_are_rejected:50`、`test_steam_cooldown_is_explicit_utc:54`、`test_utc_timestamp_matches_epoch_in_non_utc_host_timezones:62`、`test_cutoff_days_ago_is_aware_utc:66`、`test_epoch_formatting_uses_configured_timezone_not_host_timezone:79`

依赖：`datetime`、`pytest`、`time`、`utils.time`

### tests/test_transaction_sale_price_semantics.py

SHA-256：`519042feb340686602c2fa370ee5882657794d77550fb7d7da3fd2204e9f8e31`

符号：`test_manual_sale_proceeds_are_normalized_to_legacy_gross_price:4`、`test_legacy_sale_price_clients_remain_gross_and_are_not_converted:29`、`test_explicit_sale_proceeds_take_precedence_for_mixed_version_clients:52`、`test_holdings_dialog_submits_net_proceeds_with_explicit_semantics:76`

依赖：`app.routes`、`pathlib`

### tests/test_transactions_delist.py

SHA-256：`2171b49223a29038f83a46183100254558acd4521a5d436ec98a885407dbcd6e`

符号：`_listing:9`、`test_delist_failure_is_structured_instead_of_internal_server_error:22`、`test_delist_resolves_and_updates_by_stable_database_id:40`、`test_delist_rejects_stale_database_id_without_calling_steam:92`、`test_delist_converts_unexpected_steam_exception_to_business_error:111`、`test_delist_reports_remote_success_when_local_update_fails:129`、`test_delist_rejects_obvious_failure_after_http_200:165`、`Response:172`、`__init__:175`、`json:178`、`Session:181`、`__init__:182`、`get:187`、`post:213`、`test_inventory_query_distinguishes_failure_from_empty_inventory:241`、`FailedSession:244`、`get:245`、`EmptyResponse:248`、`json:252`、`EmptySession:255`、`get:256`

依赖：`app`、`app.routes`、`pathlib`、`pytest`、`sys`

### utils/__init__.py

SHA-256：`7eb70257593da06f682a3ddda54a9d260d4fc514f645237f5ca74b08f8da61a6`

符号：包入口，无定义

依赖：无 import

### utils/delay.py

SHA-256：`5bf70029ca35c24294a245ea4a930a9dbe8d18420b95d2134595c9c4982e8a2e`

符号：`jittered_sleep:3`

依赖：`app.state`、`random`、`time`

### utils/money.py

SHA-256：`8185973f15a7218f10b204b5ddb4684d2bfd7674280d47f56e506a690a86942e`

符号：`calculate_fee:32`、`get_total_with_fees:38`、`to_valid_market_price:45`、`get_item_price_from_total:58`、`usd_to_cny:81`、`_load_exchange_rates:83`、`apply_currency:105`、`yuan_to_cents:122`、`list_price_display_to_cents:126`、`cents_to_yuan:136`

依赖：`json`、`math`、`pathlib`、`time`、`typing`

### utils/network_check.py

SHA-256：`147c60fe7531c9c83732704e3bf078020f9f9ff29980c26573d5777207a7ffad`

符号：`_ping_baidu:13`、`NetworkChecker:24`、`__init__:25`、`report_success:39`、`report_failure:43`、`wait_until_online:72`、`is_offline:97`、`_log:101`、`get_network_checker:113`

依赖：`requests`、`threading`、`time`、`typing`

### utils/proxy_manager.py

SHA-256：`ab3443fccf7cbaf16a9ba704e89599cca04b1c52c6d8cbc5b3df26a13f85e071`

符号：`_load_proxy_pool_cfg:14`、`_build_proxy_url:21`、`_pm_log:29`、`test_one_proxy:35`、`ProxyManager:63`、`__init__:64`、`_reload:79`、`_sync_cycle:88`、`reload:95`、`warmup:108`、`get_strategy:157`、`is_proxy_enabled:159`、`get_next_proxy_dict:161`、`should_use_proxy_on_failure:174`、`should_always_use_proxy:177`、`get_proxies_for_request:180`、`get_proxy_manager:224`

依赖：`app.config_loader`、`app.state`、`concurrent.futures`、`random`、`requests`、`threading`、`time`、`typing`

### utils/throttle.py

SHA-256：`761895fc8f12efc3af01c749a3728aa13b0056cc89c36fdc4323a174cc755cd4`

符号：`DomainThrottle:5`、`__init__:6`、`wait:9`、`get_throttle:19`

依赖：`threading`、`time`、`typing`、`utils.delay`

### utils/time.py

SHA-256：`f2bd84e2f8105fe96a20a9563440c43b849aa149203642f17361fb225ef3dd7d`

符号：`format_utc_offset:16`、`resolve_configured_timezone:22`、`now_in_configured_timezone:50`、`timestamp_in_configured_timezone:56`、`_parse_utc_offset:65`、`parse_steam_history_date:86`、`parse_steam_cooldown:100`、`cutoff_days_ago:112`、`utc_timestamp:116`

依赖：`datetime`、`re`、`time`、`typing`、`zoneinfo`

### utils/trend.py

SHA-256：`4d28b1438900029e067e1c5fe9a8e1041f5ada22e7586c28ae8e7ac317cd1e07`

符号：`_dynamic_sensitivity:2`、`calculate_trend_robust:8`

依赖：`numpy`
## AetherSwap_audit

HEAD：`4dcd80de7331b7fb1ffaddc253b7dc4d4b64ae64`；文件 157；Python 105。

本地差异（不会覆盖）：
```text
M .gitignore
 M app/config_schema.py
 M app/database.py
 M app/pipeline.py
 M app/pipeline_steps.py
 M app/receive_flow.py
 M app/routes/__init__.py
 M app/routes/inventory.py
 M app/routes/status.py
 M app/routes/transactions.py
 M app/sell_pipeline.py
 M app/services/buff_client.py
 M app/services/workers.py
 M app/state.py
 M app/strategy_engine.py
 M app/sync_sold.py
 M buff/buyer.py
 M steam/market_orders.py
 M strategy_audit.md
 M tests/test_market_orders.py
 M web/css/components.css
 M web/index.html
 M web/js/accounts.js
 M web/js/main.js
 M web/js/strategies.js
 M web/js/utils.js
?? app/candidate_report.py
?? app/routes/candidates.py
?? app/trade_ledger.py
?? requirements-dev.txt
?? tests/test_buff_cooldown_modes.py
?? tests/test_candidate_report.py
?? tests/test_candidate_routes.py
?? tests/test_trade_ledger.py
?? web/js/candidateDashboard.js
?? web/js/tradeLedger.js
```

| 路径 | 分组 |
| --- | --- |
| `.agreed_disclaimer` | 启动、包入口与工程资产 |
| `.gitignore` | 启动、包入口与工程资产 |
| `LICENSE` | 启动、包入口与工程资产 |
| `README.md` | 启动、包入口与工程资产 |
| `analysis/__init__.py` | 策略与估值 |
| `analysis/stability.py` | 策略与估值 |
| `app/__init__.py` | 启动、包入口与工程资产 |
| `app/__main__.py` | 启动、包入口与工程资产 |
| `app/accounts.py` | 账户与认证 |
| `app/api.py` | 启动、包入口与工程资产 |
| `app/candidate_report.py` | 策略与估值 |
| `app/config_loader.py` | 持久化与配置 |
| `app/config_schema.py` | 持久化与配置 |
| `app/database.py` | 持久化与配置 |
| `app/gift_engine.py` | 游戏折扣与礼物 |
| `app/inventory_cs2.py` | Steam 行情与库存协议 |
| `app/main.py` | 启动、包入口与工程资产 |
| `app/notify.py` | 启动、包入口与工程资产 |
| `app/pipeline.py` | 交易生命周期 |
| `app/pipeline_context.py` | 交易生命周期 |
| `app/pipeline_steps.py` | 交易生命周期 |
| `app/purchase_limit.py` | 策略与估值 |
| `app/receive_flow.py` | 交易生命周期 |
| `app/repair_error_records.py` | 交易生命周期 |
| `app/routes/__init__.py` | HTTP 入口 |
| `app/routes/accounts.py` | HTTP 入口 |
| `app/routes/auth.py` | 账户与认证 |
| `app/routes/candidates.py` | HTTP 入口 |
| `app/routes/config.py` | HTTP 入口 |
| `app/routes/gift.py` | 游戏折扣与礼物 |
| `app/routes/inventory.py` | HTTP 入口 |
| `app/routes/pipeline.py` | HTTP 入口 |
| `app/routes/proxy.py` | HTTP 入口 |
| `app/routes/static.py` | HTTP 入口 |
| `app/routes/status.py` | HTTP 入口 |
| `app/routes/steam_deals.py` | 游戏折扣与礼物 |
| `app/routes/strategies.py` | HTTP 入口 |
| `app/routes/transactions.py` | HTTP 入口 |
| `app/runtime_env.py` | 启动、包入口与工程资产 |
| `app/sell_pipeline.py` | 交易生命周期 |
| `app/services/__init__.py` | 基础设施与后台任务 |
| `app/services/account_region.py` | 基础设施与后台任务 |
| `app/services/analysis_client.py` | 基础设施与后台任务 |
| `app/services/buff_auth.py` | 账户与认证 |
| `app/services/buff_client.py` | BUFF 协议与执行保护 |
| `app/services/deal_cards.py` | 游戏折扣与礼物 |
| `app/services/iflow_client.py` | 行情来源 |
| `app/services/retry.py` | 基础设施与后台任务 |
| `app/services/steam_auth.py` | 账户与认证 |
| `app/services/steam_client.py` | 基础设施与后台任务 |
| `app/services/steam_deals.py` | 游戏折扣与礼物 |
| `app/services/task_queue.py` | 基础设施与后台任务 |
| `app/services/workers.py` | 基础设施与后台任务 |
| `app/shared_market.py` | Steam 行情与库存协议 |
| `app/state.py` | 持久化与配置 |
| `app/steam_confirm.py` | 账户与认证 |
| `app/steam_delist.py` | 交易生命周期 |
| `app/steam_listings.py` | 交易生命周期 |
| `app/strategy_engine.py` | 策略与估值 |
| `app/sync_sold.py` | 交易生命周期 |
| `app/trade_ledger.py` | 持久化与配置 |
| `buff/__init__.py` | BUFF 协议与执行保护 |
| `buff/buyer.py` | BUFF 协议与执行保护 |
| `buff_error_utf8.txt` | 启动、包入口与工程资产 |
| `config/__init__.py` | 持久化与配置 |
| `config/config.json` | 持久化与配置 |
| `config/strategies.json` | 持久化与配置 |
| `flags/ar.png` | 素材与样例 |
| `flags/az.png` | 素材与样例 |
| `flags/br.png` | 素材与样例 |
| `flags/cl.png` | 素材与样例 |
| `flags/cn.png` | 素材与样例 |
| `flags/hk.png` | 素材与样例 |
| `flags/id.png` | 素材与样例 |
| `flags/in.png` | 素材与样例 |
| `flags/kz.png` | 素材与样例 |
| `flags/ph.png` | 素材与样例 |
| `flags/ru.png` | 素材与样例 |
| `flags/tr.png` | 素材与样例 |
| `flags/ua.png` | 素材与样例 |
| `flags/vn.png` | 素材与样例 |
| `images/1.png` | 素材与样例 |
| `images/2.png` | 素材与样例 |
| `images/3.png` | 素材与样例 |
| `images/4.png` | 素材与样例 |
| `images/5.png` | 素材与样例 |
| `images/steam_deal_card_sample.png` | 素材与样例 |
| `output_cards/1091500_Cyberpunk 2077.png` | 素材与样例 |
| `requirements-dev.txt` | 启动、包入口与工程资产 |
| `requirements-server.txt` | 启动、包入口与工程资产 |
| `requirements.txt` | 启动、包入口与工程资产 |
| `run.py` | 启动、包入口与工程资产 |
| `steam/__init__.py` | Steam 行情与库存协议 |
| `steam/client.py` | Steam 行情与库存协议 |
| `steam/inventory.py` | Steam 行情与库存协议 |
| `steam/market.py` | Steam 行情与库存协议 |
| `steam/market_orders.py` | Steam 行情与库存协议 |
| `steam/session.py` | 账户与认证 |
| `steamdt/__init__.py` | 行情来源 |
| `steamdt/fetcher.py` | 行情来源 |
| `steamdt/models.py` | 行情来源 |
| `steamdt/parser.py` | 行情来源 |
| `strategy_audit.md` | 启动、包入口与工程资产 |
| `tests/__init__.py` | 测试与行为合同 |
| `tests/test_account_region_sync.py` | 测试与行为合同 |
| `tests/test_auth_relogin_state.py` | 测试与行为合同 |
| `tests/test_background_startup.py` | 测试与行为合同 |
| `tests/test_buff_cooldown_modes.py` | 测试与行为合同 |
| `tests/test_buff_verification.py` | 测试与行为合同 |
| `tests/test_candidate_report.py` | 测试与行为合同 |
| `tests/test_candidate_routes.py` | 测试与行为合同 |
| `tests/test_config_loader_validation.py` | 测试与行为合同 |
| `tests/test_config_routes.py` | 测试与行为合同 |
| `tests/test_config_schema.py` | 测试与行为合同 |
| `tests/test_config_schema_hardening.py` | 测试与行为合同 |
| `tests/test_currency_conversion.py` | 测试与行为合同 |
| `tests/test_database.py` | 测试与行为合同 |
| `tests/test_holdings_stats.py` | 测试与行为合同 |
| `tests/test_market_orders.py` | 测试与行为合同 |
| `tests/test_pipeline_lock.py` | 测试与行为合同 |
| `tests/test_pipeline_steps.py` | 测试与行为合同 |
| `tests/test_proxy_manager.py` | 测试与行为合同 |
| `tests/test_runtime_env.py` | 测试与行为合同 |
| `tests/test_sell_pipeline_currency_guard.py` | 测试与行为合同 |
| `tests/test_static_routes.py` | 测试与行为合同 |
| `tests/test_steam_auth_errors.py` | 测试与行为合同 |
| `tests/test_strategies.py` | 测试与行为合同 |
| `tests/test_strategy_hardening.py` | 测试与行为合同 |
| `tests/test_trade_ledger.py` | 测试与行为合同 |
| `utils/__init__.py` | 基础设施与后台任务 |
| `utils/delay.py` | 基础设施与后台任务 |
| `utils/money.py` | 策略与估值 |
| `utils/network_check.py` | 基础设施与后台任务 |
| `utils/proxy_manager.py` | 基础设施与后台任务 |
| `utils/throttle.py` | 基础设施与后台任务 |
| `utils/time.py` | 基础设施与后台任务 |
| `utils/trend.py` | 策略与估值 |
| `web/css/animations.css` | 前端 |
| `web/css/base.css` | 前端 |
| `web/css/components.css` | 前端 |
| `web/css/layout.css` | 前端 |
| `web/css/variables.css` | 前端 |
| `web/index.html` | 前端 |
| `web/js/accounts.js` | 前端 |
| `web/js/candidateDashboard.js` | 前端 |
| `web/js/console.js` | 前端 |
| `web/js/gift.js` | 前端 |
| `web/js/main.js` | 前端 |
| `web/js/proxy.js` | 前端 |
| `web/js/settings.js` | 前端 |
| `web/js/steamDeals.js` | 前端 |
| `web/js/steamGuard.js` | 前端 |
| `web/js/strategies.js` | 前端 |
| `web/js/theme.js` | 前端 |
| `web/js/tradeLedger.js` | 前端 |
| `web/js/transactions.js` | 前端 |
| `web/js/utils.js` | 前端 |

### analysis/__init__.py

SHA-256：`bae3e2ac4c76bbbf4a96a25434988ee1bb0286b93ee280ec7ff63eb60536b135`

符号：包入口，无定义

依赖：`.stability`

### analysis/stability.py

SHA-256：`d8cadc865c185c2e0ad50be2491720f4d33309030c9e81803fbb7e4d2c332d3d`

符号：`_percentile:20`、`_iqr_bounds:31`、`clean_prices_iqr:43`、`_vwap_iqr:54`、`_apply_currency:67`、`_parse_item_date:70`、`_safe_volume:76`、`_linear_regression_slope:83`、`_linear_regression_r_squared:96`、`_ema:113`、`_analyze_market_status:121`、`_daily_avg_prices_last_n:132`、`analyze_by_time:142`、`calculate_stability:340`

依赖：`collections`、`datetime`、`statistics`、`typing`、`utils.money`

### app/__init__.py

SHA-256：`7eb70257593da06f682a3ddda54a9d260d4fc514f645237f5ca74b08f8da61a6`

符号：包入口，无定义

依赖：无 import

### app/__main__.py

SHA-256：`e25aee7d26261110026fcdecf4b04298629ac98ab451a2a7d156679c3d874d3b`

符号：包入口，无定义

依赖：`app.main`

### app/accounts.py

SHA-256：`461c8705aaa2ff1a3b65d80d60cf6e0cfed29f6ca282f021be6d690b05f7c77e`

符号：`_load:7`、`_save:20`、`list_accounts:26`、`get_current_id:29`、`get_current_account:31`、`get_account:37`、`add_account:39`、`update_account:57`、`delete_account:82`、`set_current:92`、`replace_all:99`、`get_profile_dir:105`

依赖：`json`、`pathlib`、`typing`、`uuid`

### app/api.py

SHA-256：`4b33a78bc87bcbf861b7e181d46bb91034ad0edc058a45dc43efc0259ff94a5b`

符号：`_start_background_workers:28`、`_lifespan:45`、`api_runtime:50`、`api_tasks:54`、`api_task_detail:58`、`shutdown_system:65`、`_do_shutdown:66`

依赖：`app.database`、`app.routes`、`app.runtime_env`、`app.services.task_queue`、`app.services.workers`、`app.state`、`contextlib`、`fastapi`、`os`、`signal`、`sys`、`threading`、`time`

路由：
- `GET /api/runtime → api_runtime:50`
- `GET /api/tasks → api_tasks:54`
- `GET /api/tasks/{task_id} → api_task_detail:58`
- `POST /api/system/shutdown → shutdown_system:65`

### app/candidate_report.py

SHA-256：`a8511e44fc8145f4e61dc286b98cab19145eb6177720acfad17942761162698e`

符号：`_parse_history_date:11`、`build_daily_curve:18`、`_percentile:58`、`seven_day_risk:66`、`build_seven_day_stress:88`、`_candidate_decision_notes:139`、`candidate_snapshot:180`、`sanitize_candidates:280`、`build_balance_plan:284`

依赖：`__future__`、`collections`、`datetime`、`time`、`typing`、`utils.money`

### app/config_loader.py

SHA-256：`a4a08687237c8a6ee742a96f7e1e67fa569d8529da0b19ccf241fc034805a162`

符号：`_invalidate_config_cache:16`、`get_steam_credentials:21`、`get_buff_credentials:23`、`update_steam_creds:25`、`update_buff_creds:27`、`load_app_config_validated:29`、`save_app_config_validated:40`

依赖：`app.config_schema`、`config`、`threading`、`time`

### app/config_schema.py

SHA-256：`5f11f9bd78601dac96af72b78018c7044ae194b22204c93b7002d6ef0a34356b`

符号：`merge:130`、`_coerce_bool:140`、`_validate_ranges:155`、`get_app_config:201`、`validate_and_fill:203`

依赖：`copy`、`typing`、`warnings`

### app/database.py

SHA-256：`1afb46f311f83718682ef77a267dbb65dca2190a48745fe7b207058d345427ac`

符号：`_compute_wilson_score:13`、`Purchase:25`、`Sale:57`、`ItemNameId:64`、`SteamDealGame:67`、`get_engine:114`、`get_session:128`、`init_db:130`、`_add_column:135`、`_purchase_from_dict:179`、`_sale_from_dict:212`、`_purchase_to_dict:220`、`_sale_to_dict:261`、`migrate_from_json:271`、`db_append_purchase:313`、`db_get_purchases:317`、`db_append_sale:321`、`db_get_sales:325`、`db_clear_transactions:329`、`db_replace_transactions:335`、`db_delete_purchase:345`、`db_delete_sale:354`、`db_update_purchase:362`、`db_update_purchase_by_id:375`、`db_delete_purchase_by_id:389`、`db_update_sale:400`、`db_delete_sale_by_id:412`、`db_get_item_nameid:423`、`db_set_item_nameid:429`、`_game_row_to_dict:444`、`db_upsert_steam_deal:463`、`db_get_steam_deals:482`、`db_get_steam_deals_count:529`、`db_get_steam_deals_last_update:541`、`db_clear_steam_deals:548`、`db_get_steam_deals_price_snapshot:553`、`db_get_steam_deals_review_snapshot:570`、`db_get_steam_deals_by_app_ids:578`

依赖：`json`、`math`、`os`、`pathlib`、`sqlalchemy`、`sqlmodel`、`threading`、`typing`

### app/gift_engine.py

SHA-256：`8ca62db9eac1d8d7e4d73d7e35c8ccf576c4b6202dd1be183bcdb463b5c5037e`

符号：`_build_cookies:12`、`get_friend_list:18`、`_normalize_country_code:114`、`_find_country_code:121`、`_extract_store_user_config:139`、`_extract_country_code:155`、`get_base_auth_status:170`、`get_wallet_balance:191`、`extract_appid_from_url:280`、`get_all_available_editions:283`、`_encode_varint:356`、`_build_addcart_payload:363`、`_build_modify_payload:372`、`_build_remove_payload:381`、`_grpc_request:384`、`_get_cart_items:414`、`_steamid64_to_accountid:441`、`run_gift_flow:447`、`_step:458`、`_fail:460`、`_success:462`、`_do_checkout:517`

依赖：`app.state`、`base64`、`bs4`、`html`、`json`、`re`、`requests`、`steam.session`、`time`、`urllib3`、`utils.proxy_manager`

### app/inventory_cs2.py

SHA-256：`21bfad305ccd58b503f567de82239b2e76498c09a60d7d94ef438881f1da3651`

符号：`_safe_iso:8`、`_parse_cooldown:15`、`scan_cs2_inventory:35`

依赖：`config`、`datetime`、`re`、`steam.inventory`、`steam.session`、`time`、`typing`

### app/main.py

SHA-256：`e4c0612c360a9ccd53392e7d6155ebf384076b1bdc9b314fe9e7b9a60f641bed`

符号：`check_runtime_dependencies:27`、`_ensure_disclaimer:43`、`_load_webview:67`、`__getattr__:75`、`run_server:82`、`_wait_for_server_thread:92`、`_probe_host:100`、`_wait_for_server_ready:104`、`main:118`

依赖：`__future__`、`app.api`、`app.runtime_env`、`os`、`pathlib`、`socket`、`sys`、`threading`、`time`、`typing_extensions`、`uvicorn`、`webbrowser`、`webview`

### app/notify.py

SHA-256：`26d33ed58a23749a2a581c57a289b37c237d78d04e7c10fc454cc52c318e7011`

符号：`send_pushplus:9`、`notify_manual_intervention_required:93`、`build_payment_notify_content:124`、`wait_email_command:159`、`compute_holdings_stats:250`、`build_holdings_report_content:270`

依赖：`app.config_loader`、`email`、`email.header`、`imaplib`、`re`、`requests`、`threading`、`time`、`typing`、`urllib.parse`

### app/pipeline.py

SHA-256：`0ad66ebefc0507f65b837843b84450feab5a76e13cb842a45e5310504f9ae72c`

符号：`_is_in_time_window:41`、`_fetch_and_filter_deals:48`、`_process_deals_for_target:84`、`buff_log:105`、`on_entering_payment:205`、`_run_pipeline:259`、`is_pipeline_running:415`、`_run_pipeline_guarded:420`、`start_pipeline:429`、`_run_manual_candidate_purchase:440`、`on_entering_payment:472`、`start_manual_candidate_purchase:501`、`run_manual:506`

依赖：`app.candidate_report`、`app.config_loader`、`app.config_schema`、`app.notify`、`app.pipeline_context`、`app.pipeline_steps`、`app.sell_pipeline`、`app.services.analysis_client`、`app.services.buff_client`、`app.services.iflow_client`、`app.services.steam_client`、`app.state`、`app.strategy_engine`、`buff.buyer`、`datetime`、`pathlib`、`steamdt.models`、`sys`、`threading`、`time`、`utils.delay`、`utils.money`、`utils.network_check`、`utils.proxy_manager`、`uuid`

### app/pipeline_context.py

SHA-256：`bb12da81b275792bef1555a45f38ee53e552a455c2b8601bbd4919cdf4e67e0b`

符号：`PipelineContext:4`、`__init__:5`、`log:10`、`debug:13`、`set_status:17`、`is_stop_requested:20`、`wait_retry:23`

依赖：`time`

### app/pipeline_steps.py

SHA-256：`89631959e60827f706efc2aaaec4ee47b5ee5a09c904c9c465eaedee2ae58e83`

符号：`_as_optional_float:33`、`_purchase_audit_fields:40`、`_order_key:61`、`_is_temp_blocked_order:64`、`_mark_payment_unavailable_order:69`、`_describe_buff_order_detail:77`、`_fetch_steam_sell_data:87`、`_check_buff_price:168`、`_adjust_ref_price_for_daily_high:207`、`_compute_sell_pressure_from_orders:262`、`_compute_sell_pressure:299`、`_fetch_smart_market_price:310`、`_parse_threshold:320`、`_goods_id_from_buff_url:329`、`_log_stability_rejection:339`、`filter_iflow_rows:363`、`_check_sell_pressure_precheck:432`、`_check_max_discount_precheck:454`、`_build_buy_strategy_outputs:484`、`_passes_custom_buy_modules:529`、`pick_stable_item:567`、`_do_payment_notify_and_wait:842`、`_email_waiter:907`、`_do_batch_wait_finalize_and_append:1001`、`_do_wait_payment_and_append:1076`、`lock_and_confirm_payment:1137`、`_try_single_buy:1315`、`_try_batch_buy:1370`

依赖：`app.candidate_report`、`app.config_loader`、`app.notify`、`app.services.analysis_client`、`app.services.buff_client`、`app.services.iflow_client`、`app.services.steam_client`、`app.state`、`app.strategy_engine`、`buff.buyer`、`datetime`、`re`、`statistics`、`steam.market_orders`、`steam.session`、`threading`、`time`、`typing`、`utils.delay`、`utils.money`、`utils.time`

### app/purchase_limit.py

SHA-256：`c0044fe06f7e675a08ccd4c0c5f7d0f2db5999dd863ad08b8e22a527c5cae2c4`

符号：`calculate_safe_purchase_limit:1`

依赖：无 import

### app/receive_flow.py

SHA-256：`32349ba40363378f3fedd374f09496f204ca0e909a74ef4eff613e660a3df0ef`

符号：`steam_request:6`、`_cookies_str_to_dict:25`、`fetch_buff_steam_trade:33`、`accept_steam_trade_offer:95`、`_call:116`、`_match_purchase_for_item:139`、`try_receive_once:173`、`_do_update:209`

依赖：`app.trade_ledger`、`requests`、`time`、`typing`、`urllib3`、`utils.delay`、`utils.proxy_manager`

### app/repair_error_records.py

SHA-256：`50f46acc15f00549e6af81b12dd3b3b1364b470fa77086cc6bfe0ed56e27b461`

符号：`_cookies_to_dict:25`、`_fetch_sold_with_names:35`、`_currency_code_from_price_text:65`、`_norm_name:138`、`_build_merged:145`、`_record_name_counts:173`、`_gather_candidates_for_record_name:180`、`_filter_list_by_record_names:191`、`_pick_candidate:199`、`_apply_candidate:205`、`_clear_assetids:226`、`_refill_from_list:236`、`run:250`、`main:333`、`log:334`

依赖：`app.inventory_cs2`、`app.state`、`app.steam_listings`、`bs4`、`collections`、`json`、`pathlib`、`re`、`requests`、`sys`、`time`、`unicodedata`、`urllib.parse`、`urllib3`

### app/routes/__init__.py

SHA-256：`b1f18275ecb589a9b6e595428eee19c54f7e73b42395ddd3d4bd4b57f07c5a10`

符号：`register_routes:18`

依赖：`app.routes.accounts`、`app.routes.auth`、`app.routes.candidates`、`app.routes.config`、`app.routes.gift`、`app.routes.inventory`、`app.routes.pipeline`、`app.routes.proxy`、`app.routes.static`、`app.routes.status`、`app.routes.steam_deals`、`app.routes.strategies`、`app.routes.transactions`、`fastapi`

### app/routes/accounts.py

SHA-256：`206e0baf6edc70649c9cf81bfecd812e10fcbdaf7c2167a64dfc71860fb87bab`

符号：`AccountBody:18`、`AccountUpdateBody:24`、`api_list_accounts:31`、`api_add_account:37`、`api_update_account:47`、`api_delete_account:64`、`api_set_current_account:68`、`api_verify_account:72`

依赖：`app.accounts`、`app.services.account_region`、`app.services.steam_auth`、`app.state`、`fastapi`、`pydantic`、`typing`

路由：
- `GET /api/accounts → api_list_accounts:31`
- `POST /api/accounts → api_add_account:37`
- `PUT /api/accounts/{account_id} → api_update_account:47`
- `DELETE /api/accounts/{account_id} → api_delete_account:64`
- `POST /api/accounts/{account_id}/set_current → api_set_current_account:68`
- `POST /api/accounts/{account_id}/verify → api_verify_account:72`

### app/routes/auth.py

SHA-256：`c45120cc2f547b0ef8e60b4323f829eded845c69a1c2e35b530d567e436960cb`

符号：`_manual_cookie_required_response:40`、`ReloginFinishBody:50`、`ManualCookieBody:52`、`_normalize_cookie_input:56`、`_cookie_value:70`、`_cookie_header_from_browser:79`、`_has_browser_cookie:87`、`_steam_id_from_cookie_str:94`、`_looks_like_browser_launch_error:101`、`_should_retry_browser_launch:116`、`_friendly_browser_launch_error:130`、`_launch_relogin_context:148`、`_maybe_resume_after_buff_cookie_update:187`、`_relogin_worker:203`、`_relogin_start:303`、`_relogin_finish:339`、`_normalize_secret:351`、`_generate_steam_guard_code:353`、`api_auth_steam_relogin_start:373`、`api_auth_steam_relogin_finish:376`、`api_auth_buff_relogin_start:379`、`api_auth_buff_relogin_finish:382`、`api_auth_manual_cookie:385`、`api_steam_guard:412`

依赖：`app.accounts`、`app.config_loader`、`app.pipeline`、`app.runtime_env`、`app.services.steam_auth`、`app.state`、`base64`、`fastapi`、`hashlib`、`hmac`、`pathlib`、`playwright.sync_api`、`pydantic`、`re`、`shutil`、`struct`、`tempfile`、`threading`、`time`、`typing`

路由：
- `POST /api/auth/steam/relogin_start → api_auth_steam_relogin_start:373`
- `POST /api/auth/steam/relogin_finish → api_auth_steam_relogin_finish:376`
- `POST /api/auth/buff/relogin_start → api_auth_buff_relogin_start:379`
- `POST /api/auth/buff/relogin_finish → api_auth_buff_relogin_finish:382`
- `POST /api/auth/{relogin_type}/manual_cookie → api_auth_manual_cookie:385`
- `GET /api/steam_guard → api_steam_guard:412`

### app/routes/candidates.py

SHA-256：`e633d8536025bf276b1e2cf90620b88a6310c007d434cfc1bc3e47b606d41f17`

符号：`CandidateSelection:40`、`PurchaseCandidatesBody:45`、`BalancePlanBody:51`、`PreflightCandidatesBody:56`、`api_candidates:61`、`api_candidates_clear:77`、`api_candidates_plan:85`、`api_candidate_research:95`、`_selection_key:133`、`api_candidates_preflight:138`、`_fetch_live_app_price:291`、`api_candidate_game_search:322`、`api_candidates_purchase:419`

依赖：`app.candidate_report`、`app.config_loader`、`app.config_schema`、`app.pipeline`、`app.pipeline_steps`、`app.routes.steam_deals`、`app.services.buff_client`、`app.services.steam_client`、`app.state`、`app.strategy_engine`、`concurrent.futures`、`fastapi`、`pydantic`、`requests`、`threading`、`time`、`typing`、`urllib.parse`、`uuid`

路由：
- `GET /api/candidates → api_candidates:61`
- `POST /api/candidates/clear → api_candidates_clear:77`
- `POST /api/candidates/plan → api_candidates_plan:85`
- `GET /api/candidates/research → api_candidate_research:95`
- `POST /api/candidates/preflight → api_candidates_preflight:138`
- `GET /api/candidates/game-search → api_candidate_game_search:322`
- `POST /api/candidates/purchase → api_candidates_purchase:419`

### app/routes/config.py

SHA-256：`67843296fb978fcd65f9df595d64c5205493c67d715c434db6912f709c6284ee`

符号：`ConfigBody:16`、`ImportFullBody:18`、`api_get_config:25`、`api_save_config:28`、`api_data_init:37`、`_shutdown:145`、`api_export_full:154`、`api_export_full_download:169`、`api_import_full:191`、`api_holdings_report_send:207`

依赖：`app.accounts`、`app.config_loader`、`app.database`、`app.services.workers`、`app.state`、`config`、`datetime`、`fastapi`、`fastapi.responses`、`glob`、`json`、`os`、`pathlib`、`pydantic`、`shutil`、`sqlmodel`、`threading`、`time`

路由：
- `GET /api/config → api_get_config:25`
- `POST /api/config → api_save_config:28`
- `POST /api/data/init → api_data_init:37`
- `GET /api/export_full → api_export_full:154`
- `GET /api/export_full/download → api_export_full_download:169`
- `POST /api/import_full → api_import_full:191`
- `POST /api/holdings_report/send → api_holdings_report_send:207`

### app/routes/gift.py

SHA-256：`089902b21e7426536acc3ee845611c1d92f4fc0674d2f974c1bd2c9befae7f08`

符号：`_new_task:12`、`_append_progress:20`、`_get_task:28`、`_get_cookies_raw:32`、`_get_steam_id:36`、`_check_cookies:40`、`api_get_friends:49`、`EditionsBody:62`、`api_get_editions:65`、`GiftBody:78`、`api_gift_send:83`、`_run:90`、`api_gift_task:107`、`api_gift_balance:113`、`api_gift_debug_config:124`

依赖：`app`、`app.config_loader`、`fastapi`、`pydantic`、`threading`、`typing`、`uuid`

路由：
- `GET /api/gift/friends → api_get_friends:49`
- `POST /api/gift/editions → api_get_editions:65`
- `POST /api/gift/send → api_gift_send:83`
- `GET /api/gift/task/{task_id} → api_gift_task:107`
- `GET /api/gift/balance → api_gift_balance:113`
- `GET /api/gift/debug-config → api_gift_debug_config:124`

### app/routes/inventory.py

SHA-256：`c50366c7e6f25d04b22f2206d3ba55880abe6836a17d6407c0ce7763d59e30b4`

符号：`_get_steam_smart_price_cny:10`、`_enrich_inventory_with_steam_prices:12`、`_try_steam_auto_relogin:36`、`_sync_purchase_unlocks:39`、`api_inventory:44`、`api_market_prices:113`

依赖：`app.config_loader`、`app.inventory_cs2`、`app.pipeline`、`app.services.steam_auth`、`app.shared_market`、`app.state`、`app.trade_ledger`、`fastapi`、`time`

路由：
- `GET /api/inventory → api_inventory:44`
- `GET /api/market-prices → api_market_prices:113`

### app/routes/pipeline.py

SHA-256：`f7966b56895006d3701115139c2ec5d9204b581765d8b6edfc2683eb11809e84`

符号：`ConfigBody:7`、`api_pipeline_start:10`、`api_pipeline_stop:16`

依赖：`app.pipeline`、`app.state`、`fastapi`、`pydantic`

路由：
- `POST /api/pipeline/start → api_pipeline_start:10`
- `POST /api/pipeline/stop → api_pipeline_stop:16`

### app/routes/proxy.py

SHA-256：`30df4a3d5f23799c51d28f113e15dbf82a55f0342f03157fa4d7b9fb530dc7d6`

符号：`ProxyEntry:10`、`ProxyPoolConfig:15`、`ProxyPoolBody:22`、`api_get_proxy_config:25`、`api_save_proxy_config:29`、`api_test_proxies:50`、`api_clear_proxies:81`、`api_fetch_webshare:95`

依赖：`app.config_loader`、`app.state`、`concurrent.futures`、`fastapi`、`pydantic`、`requests`、`time`、`typing`、`utils.proxy_manager`

路由：
- `GET /api/proxy/config → api_get_proxy_config:25`
- `POST /api/proxy/config → api_save_proxy_config:29`
- `POST /api/proxy/test → api_test_proxies:50`
- `POST /api/proxy/clear → api_clear_proxies:81`
- `POST /api/proxy/webshare → api_fetch_webshare:95`

### app/routes/static.py

SHA-256：`c6c5e16cab4e5f592036659706eb73f5bd6c0bfe9b66c0322b10c27f5bc5c803`

符号：`_safe_web_file:10`、`index:21`、`static_or_index:27`

依赖：`fastapi`、`fastapi.responses`、`pathlib`、`typing`

路由：
- `GET / → index:21`
- `GET /{path:path} → static_or_index:27`

### app/routes/status.py

SHA-256：`58234d0f30fd13ee1fac59da6522238b75d9669cec9930b74f47658c6bb56f3e`

符号：`ConfirmBody:17`、`api_status:21`、`api_log:28`、`api_log_clear:31`、`api_log_export:35`、`fmt_time:41`、`api_plan:52`、`api_pending_payment:55`、`api_confirm_payment:58`

依赖：`app.state`、`config`、`datetime`、`fastapi`、`pathlib`、`pydantic`、`typing`

路由：
- `GET /api/status → api_status:21`
- `GET /api/log → api_log:28`
- `POST /api/log/clear → api_log_clear:31`
- `POST /api/log/export → api_log_export:35`
- `GET /api/plan → api_plan:52`
- `GET /api/pending_payment → api_pending_payment:55`
- `POST /api/confirm_payment → api_confirm_payment:58`

### app/routes/steam_deals.py

SHA-256：`a2116f62ef13e9e938225612e94f8d5b90d94f72a500d30f009f70e64de34358`

符号：`_load_exchange_rates:35`、`_get_rates:45`、`_invalidate_sort_cache:58`、`_build_sort_index:61`、`_parse_price:129`、`_to_cny:165`、`api_get_steam_deals:184`、`api_exchange_rates:255`、`api_fetch_steam_deals:260`、`api_steam_deals_status:289`、`api_generate_deal_card:309`

依赖：`app.config_loader`、`app.database`、`app.services.deal_cards`、`app.services.steam_deals`、`fastapi`、`json`、`math`、`pathlib`、`re`、`sqlmodel`、`threading`、`time`、`typing`

路由：
- `GET /api/steam-deals → api_get_steam_deals:184`
- `GET /api/steam-deals/exchange-rates → api_exchange_rates:255`
- `POST /api/steam-deals/fetch → api_fetch_steam_deals:260`
- `GET /api/steam-deals/status → api_steam_deals_status:289`
- `POST /api/steam-deals/generate-card/{app_id} → api_generate_deal_card:309`

### app/routes/strategies.py

SHA-256：`87b3de218c8371c8c0f17933dbfb3ab07e9876df652d681acfff76e3cf8a00ff`

符号：`StrategyBody:20`、`ActivateBody:24`、`ImportBody:28`、`ModuleImportBody:32`、`SimulateBody:36`、`_error:41`、`api_strategies:46`、`api_save_strategy:51`、`api_delete_strategy:59`、`api_activate_strategy:68`、`api_import_strategy:76`、`api_export_strategy:84`、`api_import_strategy_module:92`、`api_simulate_strategy:100`

依赖：`app.strategy_engine`、`fastapi`、`pydantic`

路由：
- `GET /api/strategies → api_strategies:46`
- `POST /api/strategies → api_save_strategy:51`
- `DELETE /api/strategies/{strategy_id} → api_delete_strategy:59`
- `POST /api/strategies/{strategy_id}/activate → api_activate_strategy:68`
- `POST /api/strategies/import → api_import_strategy:76`
- `GET /api/strategies/{strategy_id}/export → api_export_strategy:84`
- `POST /api/strategy-modules/import → api_import_strategy_module:92`
- `POST /api/strategies/simulate → api_simulate_strategy:100`

### app/routes/transactions.py

SHA-256：`1a115f8a864acb2bb8c5ab3fc8efa415da5e1066ef9630d2ce12fb9ceb410675`

符号：`AddPurchaseBody:27`、`TransactionUpdateBody:34`、`_name_from_steam_link:46`、`_get_steam_smart_price_cny:52`、`_fetch_steam_lowest_cny:54`、`_enrich_purchases_with_current_prices:60`、`api_purchases:80`、`api_add_purchase:83`、`api_transactions:119`、`api_trade_ledger:171`、`api_delete_transaction:176`、`api_update_transaction:185`、`api_stats:222`、`api_delist_purchase:255`、`log_fn:267`、`api_sync_sold_from_history:294`、`log_fn:296`、`api_repair_error_records:313`、`log_fn:315`

依赖：`app.config_loader`、`app.repair_error_records`、`app.shared_market`、`app.state`、`app.steam_delist`、`app.sync_sold`、`app.trade_ledger`、`fastapi`、`pydantic`、`steam.client`、`time`、`typing`

路由：
- `GET /api/purchases → api_purchases:80`
- `POST /api/purchase → api_add_purchase:83`
- `GET /api/transactions → api_transactions:119`
- `GET /api/trade-ledger → api_trade_ledger:171`
- `DELETE /api/transaction → api_delete_transaction:176`
- `PUT /api/transaction → api_update_transaction:185`
- `GET /api/stats → api_stats:222`
- `POST /api/purchase/{idx}/delist → api_delist_purchase:255`
- `POST /api/sync_sold_from_history → api_sync_sold_from_history:294`
- `POST /api/repair_error_records → api_repair_error_records:313`

### app/runtime_env.py

SHA-256：`d4d849da3d2caf14bbe65df1118a331b4a983afc1ac64b30d974bfe89de6e7c8`

符号：`RuntimeProfile:16`、`as_dict:27`、`_clean:31`、`parse_bool:35`、`env_bool:46`、`_env_value:51`、`has_graphical_display:60`、`get_runtime_profile:71`、`get_bind_host:127`、`get_port:135`、`get_local_url:146`、`get_external_url_hint:151`

依赖：`__future__`、`dataclasses`、`os`、`sys`、`typing`

### app/sell_pipeline.py

SHA-256：`c87350d93ce76280abc683fc9f8142f1bced97193ce1f7c0444ccf272f4c2777`

符号：`_steam_latest_price_and_trend:33`、`_load_rate_map:73`、`_record_listing_success:87`、`_resolve_steam_session:109`、`_get_inventory:128`、`_build_listing_plan:156`、`_find_buy_record:381`、`_submit_listings:393`、`_do_list:419`、`_auto_confirm_listings:470`、`_run_sell_phase_impl:498`、`_run_sell_phase:607`、`run_sell_phase_on_inventory_update:621`

依赖：`app.accounts`、`app.config_loader`、`app.config_schema`、`app.inventory_cs2`、`app.pipeline_context`、`app.services.account_region`、`app.services.steam_auth`、`app.services.steam_client`、`app.state`、`app.steam_confirm`、`app.steam_listings`、`app.strategy_engine`、`collections`、`datetime`、`json`、`pathlib`、`steam.market`、`steam.market_orders`、`steam.session`、`threading`、`time`、`typing`、`utils.delay`、`utils.money`、`utils.time`、`utils.trend`

### app/services/__init__.py

SHA-256：`b7f31b41f81376f7be769096888e1f6378998c3f9dfca809f40b3a7fd948ee1f`

符号：包入口，无定义

依赖：`app.services.analysis_client`、`app.services.buff_client`、`app.services.iflow_client`、`app.services.retry`、`app.services.steam_client`

### app/services/account_region.py

SHA-256：`3f26a16a8eda488d3af26d5c9801f32a84e747914661960f5fa469d81d104cd3`

符号：`_short_error:47`、`_region_from_settlement_currency:53`、`refresh_account_region_currency:59`

依赖：`app.accounts`、`app.config_loader`、`app.gift_engine`、`time`、`typing`

### app/services/analysis_client.py

SHA-256：`8bc852e2dac331db6aaf62000122bc61cfcd209db369af576b7ad4b3a43477a0`

符号：`StabilityAnalyzer:4`、`__init__:5`、`analyze:7`

依赖：`analysis`、`typing`、`utils.money`

### app/services/buff_auth.py

SHA-256：`77d6902d77d35c8665c1d7972049a7a6a82fffa467b82201a8da482f905c19df`

符号：`try_buff_auto_relogin:11`、`_try_buff_auto_relogin_impl:22`

依赖：`app.config_loader`、`app.notify`、`app.state`、`pathlib`、`playwright.sync_api`、`threading`、`time`

### app/services/buff_client.py

SHA-256：`1536ba0a9324158201ea9120d46665cb07a806efbf80c5b1e8ff5958f659d724`

符号：`resolve_cooldown_mode:22`、`cooldown_mode_label:30`、`cooldown_mode_allows_cooldown:35`、`get_sell_orders_for_cooldown_mode:39`、`count_lowest_price_orders:63`、`first_order_at_price:81`、`BuffClient:90`、`__init__:91`、`get_sell_orders:101`、`get_goods_steam_price_cny:103`、`ask_seller_to_send:105`、`lock_and_get_pay_url:108`、`try_batch_buy:118`、`batch_buy_find_and_finalize:144`、`is_order_paid:172`、`cancel_order:176`、`get_order_state:180`、`get_order_detail:184`、`create_buff_client_from_config:187`

依赖：`app.services.retry`、`buff.buyer`、`typing`

### app/services/deal_cards.py

SHA-256：`3ce5810953eb67cf723476bacc4d0a37bf4fed70e6ad566da624e98da4628c76`

符号：`_load_font:50`、`_download_banner:73`、`_get_flag:95`、`draw_rounded_rect:121`、`get_text_size:137`、`draw_text_middle:146`、`draw_text_left:152`、`parse_price_to_rmb:158`、`wrap_text:207`、`generate_card:228`

依赖：`PIL`、`app.database`、`argparse`、`datetime`、`io`、`json`、`os`、`pathlib`、`re`、`requests`、`sqlmodel`、`sys`、`time`、`typing`、`utils.proxy_manager`

### app/services/iflow_client.py

SHA-256：`3690bf14c910774fe6770ec33d8b139a26ab2daed3b6eb20c018db934e8547ca`

符号：`IflowClient:26`、`__init__:33`、`fetch:36`、`fetch_iflow_rows:79`

依赖：`math`、`steamdt.fetcher`、`steamdt.models`、`typing`

### app/services/retry.py

SHA-256：`4d9b54338ae504bc15deedf15eb30d538d494b0f83d87fea07a900f4e981d493`

符号：`with_retry:8`、`decorator:15`、`inner:17`

依赖：`functools`、`logging`、`time`、`typing`、`utils.delay`

### app/services/steam_auth.py

SHA-256：`e3deb12a42e684240ca0696c302306ff76ca6aa7990d23ec367918b6c9d7e716`

符号：`_verify_steam_cookies_valid:26`、`fetch_steam_profile_via_api:77`、`_get_shared_secret:149`、`_build_steam_guard_dict:158`、`_short_error_detail:183`、`_classify_steam_login_exception:189`、`_do_steampy_login:225`、`_bypass_ssl:238`、`_extract_creds_from_cookie_dict:282`、`try_steam_auto_relogin:295`、`_try_steam_auto_relogin_impl:306`、`verify_steam_auto_login:375`

依赖：`app.accounts`、`app.config_loader`、`app.notify`、`app.state`、`json`、`re`、`requests`、`requests.utils`、`steampy.client`、`threading`、`time`、`typing`、`urllib3`

### app/services/steam_client.py

SHA-256：`e3738861d733a353e8b592e24a53804e3c3483a0f36465670faf390a0ded0673`

符号：`SteamClient:14`、`__init__:15`、`fetch_history:24`、`_fetch_history_impl:44`、`market_hash_name_from_listing_url:79`、`create_steam_client:81`

依赖：`app.config_loader`、`app.services.retry`、`app.state`、`steam.client`、`time`、`typing`、`utils.delay`、`utils.proxy_manager`

### app/services/steam_deals.py

SHA-256：`827ba1156dc447f3e08052f5379aff67c518e9a8d2d913d172fb6561543033be`

符号：`get_fetch_state:37`、`_update_state:40`、`_build_valid_proxies:43`、`_test:70`、`_fetch_with_proxy:91`、`_get_discounted_appids:106`、`_fetch_region_data:173`、`_fetch_historical_low:205`、`_get_deal_status:221`、`_process_single_game:236`、`run_fetch:318`

依赖：`app.config_loader`、`app.database`、`concurrent.futures`、`json`、`pathlib`、`random`、`re`、`requests`、`threading`、`time`、`typing`、`urllib3`

### app/services/task_queue.py

SHA-256：`a4afe5afbfb99523fe45e228169b13a3100f57c599bfd2e67d5f67f7409730f7`

符号：`TaskStatus:15`、`TaskInfo:22`、`TaskQueue:33`、`__init__:42`、`submit:47`、`get_task:87`、`list_tasks:93`、`active_count:97`、`shutdown:100`、`_run_with_retries:102`、`_trim_history:132`、`_info_to_dict:144`、`get_task_queue:159`

依赖：`concurrent.futures`、`dataclasses`、`enum`、`threading`、`time`、`traceback`、`typing`、`uuid`

### app/services/workers.py

SHA-256：`0de170add4c3fd021bf3c91323b6448176caa58e5ff3eb83cc07f28b2df7bd2e`

符号：`_worker_alert:32`、`_load_last_pl_pct:49`、`_save_last_pl_pct:59`、`_enrich_purchases_with_current_prices:66`、`run_holdings_report_once:85`、`_build_out:96`、`_enriched_holdings:105`、`holdings_report_worker:136`、`_fetch_exchange_rates:162`、`_save_exchange_rates:190`、`exchange_rate_worker:203`、`receive_worker:236`、`listing_check_worker:262`、`_currency_code_from_price_text:332`、`_detect_account_currency_from_history:349`、`_sync_account_profile_and_region:395`、`sync_account_region_worker:421`、`session_keepalive_worker:459`

依赖：`app.accounts`、`app.config_loader`、`app.inventory_cs2`、`app.notify`、`app.receive_flow`、`app.sell_pipeline`、`app.services.account_region`、`app.services.buff_auth`、`app.services.steam_auth`、`app.shared_market`、`app.state`、`app.steam_listings`、`bs4`、`datetime`、`json`、`pathlib`、`requests`、`time`、`typing`、`utils.proxy_manager`

### app/shared_market.py

SHA-256：`55b15f8a17d67c3cfda8f6032a5be3edc9be0422b4f042e1a4eecaaafc7854c5`

符号：`get_steam_smart_price_cny:9`、`batch_fetch_prices:28`、`_fetch_one:42`

依赖：`app.config_loader`、`concurrent.futures`、`steam.market_orders`、`steam.session`、`typing`

### app/state.py

SHA-256：`34606fc56e00114c168fe08fceda4f09639a2e03f53ad07bde076308368b42a9`

符号：`State:22`、`__init__:45`、`append_purchase:69`、`get_purchases:71`、`append_sale:73`、`get_sales:75`、`clear_transactions:77`、`reload_transactions:79`、`replace_transactions:81`、`delete_purchase:83`、`delete_sale:85`、`update_purchase:87`、`update_purchase_by_id:89`、`delete_purchase_by_id:91`、`update_sale:93`、`set_status:95`、`get_status:106`、`is_steam_background_allowed:121`、`set_buff_auth_expired:126`、`set_buff_verification_required:132`、`log:138`、`get_log:149`、`set_pending_payment:155`、`get_pending_payment:158`、`wait_payment_confirm:161`、`confirm_payment:180`、`get_payment_confirm_reason:185`、`request_stop:188`、`clear_stop:193`、`is_stop_requested:196`、`set_plan:199`、`get_plan:202`、`set_inventory:205`、`get_inventory:208`、`set_deal_candidates:211`、`get_deal_candidates:215`、`clear_deal_candidates:221`、`reset_candidate_rejections:223`、`record_candidate_rejection:226`、`get_candidate_rejections:230`、`clear_log:233`、`replace_log:236`、`get_state:253`、`set_state:259`、`set_status:263`、`get_status:265`、`is_steam_background_allowed:267`、`set_buff_auth_expired:269`、`set_buff_verification_required:271`、`log:273`、`get_log:275`、`set_pending_payment:277`、`get_pending_payment:279`、`wait_payment_confirm:281`、`confirm_payment:283`、`get_payment_confirm_reason:285`、`request_stop:287`、`clear_stop:289`、`is_stop_requested:291`、`set_plan:293`、`get_plan:295`、`append_purchase:297`、`get_purchases:299`、`append_sale:301`、`get_sales:303`、`reload_transactions:305`、`clear_transactions:307`、`replace_transactions:309`、`delete_purchase:311`、`delete_purchase_by_id:313`、`delete_sale:315`、`delete_sale_by_id:317`、`update_purchase:319`、`update_purchase_by_id:321`、`update_sale:323`、`set_inventory:325`、`get_inventory:327`、`set_deal_candidates:329`、`get_deal_candidates:331`、`clear_deal_candidates:333`、`reset_candidate_rejections:335`、`record_candidate_rejection:337`、`get_candidate_rejections:339`、`clear_log:341`、`replace_log:343`

依赖：`app.database`、`collections`、`json`、`pathlib`、`threading`、`time`、`typing`

### app/steam_confirm.py

SHA-256：`3781dddc6ef481f0041bcb70804881e73071f869ae7ef62d980d692d69b068f1`

符号：`_cookies_to_dict:11`、`SteamConfirmer:20`、`__init__:21`、`_signature:40`、`get_confirmations:47`、`accept_all:73`、`auto_confirm_once:106`

依赖：`base64`、`hashlib`、`hmac`、`requests`、`struct`、`time`、`typing`、`urllib.parse`、`urllib3`

### app/steam_delist.py

SHA-256：`b192de399fc75968db69a168c3df723addff64c15832c15cb6539a85fa750fca`

符号：`_cookies_to_dict:22`、`_get_mylistings_api:32`、`extract_items:47`、`_extract_js_var:90`、`_get_asset_class_instance_from_market_page:123`、`_get_assetids_by_class_instance:144`、`delist_item:180`

依赖：`config`、`json`、`re`、`requests`、`steam.inventory`、`steam.session`、`time`、`typing`、`urllib3`、`utils.delay`

### app/steam_listings.py

SHA-256：`8d1a5ddb1484fce60ed957f560fab20235ad35fa5a54b29c5e2c612fe477daeb`

符号：`_get_with_retry:21`、`_cookies_to_dict:45`、`_extract_js_var:55`、`_debug:91`、`_parse_assets_730_2:94`、`_parse_sell_listings_from_html:106`、`fetch_my_listings:146`、`fetch_my_history_sold:194`、`_currency_code_from_price_text:242`

依赖：`bs4`、`json`、`pathlib`、`re`、`requests`、`time`、`typing`、`urllib3`、`utils.proxy_manager`

### app/strategy_engine.py

SHA-256：`8e5347d0b4aebb16de187948658780143b4bcecee9691ec6636171d00c4807fd`

符号：`StrategyError:619`、`_read_json:623`、`_write_json:634`、`_strategy_store:640`、`_save_strategy_store:646`、`_module_store:650`、`_save_module_store:656`、`_sanitize_value:660`、`_slugify:673`、`_custom_id:678`、`_system_by_id:682`、`_user_module_by_id:686`、`_clean_module_id_list:695`、`_clean_requires_any:701`、`_clean_logic:712`、`_coerce_bool:717`、`_clean_stage_list:734`、`_clean_conditions:749`、`_has_user_code_fields:777`、`_is_declarative_user_module:781`、`_normalize_user_module_kind:790`、`_normalize_max_instances:797`、`_apply_module_defaults:804`、`_module_by_id:829`、`list_custom_strategies:846`、`list_strategies:850`、`get_strategy:854`、`_legacy_sell_id:861`、`get_active_strategy_ids:869`、`_normalize_step:881`、`normalize_strategy:890`、`_validate_step_params:918`、`_validate_user_module_contract:966`、`validate_strategy:986`、`_system_strategy_with_submitted_params:1104`、`_save_system_strategy_params:1127`、`save_strategy:1148`、`delete_strategy:1163`、`activate_strategy:1187`、`import_strategy:1210`、`export_strategy:1218`、`import_user_module:1230`、`_enabled_steps:1294`、`_current_param_values:1298`、`_hydrate_strategy_params:1377`、`_step_params:1408`、`_enabled_module_ids:1415`、`_derive_sell_strategy_number:1419`、`apply_strategy_to_config:1430`、`is_strategy_module_enabled:1546`、`get_strategy_module_params:1553`、`_available_data_catalog:1562`、`_get_nested:1571`、`_resolve_output_path:1590`、`_resolve_path:1603`、`_as_number:1618`、`_compare_values:1627`、`_evaluate_condition:1669`、`_evaluate_declarative_module:1690`、`_module_stage_matches:1719`、`evaluate_strategy_runtime_modules:1726`、`_sample_strategy_context:1765`、`_simulate_builtin_step:1786`、`get_strategy_payload:1802`、`simulate_strategy:1824`

依赖：`app.config_loader`、`config`、`copy`、`json`、`pathlib`、`re`、`time`、`typing`

内置策略模块（参数仅列字段，不读取运行配置）：

| ID | 名称 | 参数 |
| --- | --- | --- |
| `buy.steamdt_top_n` | SteamDT 取前 N 条 | iflow_top_n |
| `buy.exclude_keywords` | 关键词排除 | exclude_keywords |
| `buy.basic_candidate_filter` | 基础候选校验 |  |
| `buy.buff_realtime_price` | Buff 实时卖单 | cooldown_mode |
| `buy.steam_sell_depth` | Steam 卖单深度 |  |
| `guard.max_discount` | 最高折扣 | max_discount |
| `guard.sell_pressure` | 卖压保护 | sell_pressure_orders_n, sell_pressure_threshold |
| `guard.history_stability` | 历史稳定性（兼容） | days, cv_threshold, r2_threshold, min_daily_trades, price_percentile_ceil, r2_rising_threshold, slope_pct_ceil, ma_deviation_ceil, last_price_ma30_ceil, slope_stable_floor, price_percentile_ceil_rising, use_vwap |
| `guard.history_data_window` | 历史数据窗口 | days, min_daily_trades, use_vwap |
| `guard.volatility_cv` | 波动率过滤 | cv_threshold |
| `guard.trend_quality` | 趋势质量过滤 | r2_threshold, r2_rising_threshold, slope_pct_ceil, slope_stable_floor |
| `guard.price_position` | 价格位置过滤 | price_percentile_ceil, price_percentile_ceil_rising, ma_deviation_ceil, last_price_ma30_ceil |
| `guard.safe_purchase_limit` | 安全采购上限（兼容） | safe_purchase_hard_qty_cap, safe_purchase_liquidity_ratio, safe_purchase_low_price_threshold, safe_purchase_low_price_penalty, safe_purchase_low_price_hard_cap |
| `guard.purchase_hard_cap` | 单品硬数量上限 | safe_purchase_hard_qty_cap |
| `guard.purchase_liquidity_cap` | 流动性采购上限 | safe_purchase_liquidity_ratio |
| `guard.low_price_purchase_guard` | 低价采购惩罚 | safe_purchase_low_price_threshold, safe_purchase_low_price_penalty, safe_purchase_low_price_hard_cap |
| `guard.held_same_item_guard` | 同名持仓扣减 |  |
| `guard.target_balance` | 目标余额保护 | target_balance |
| `action.buff_lock_pay` | Buff 锁单付款 |  |
| `sell.sellable_inventory_filter` | 可出售库存 |  |
| `guard.max_listings_per_item` | 同名在售上限 | max_listings_per_item |
| `pricing.steam_wall_gap` | 墙+断层一体定价 | sell_price_wall_volume, sell_price_max_ignore_volume, sell_price_offset |
| `pricing.steam_wall_price` | 价格墙定价 | sell_price_wall_volume, sell_price_max_ignore_volume |
| `pricing.price_offset` | 上架价补偿 | sell_price_offset |
| `guard.rising_trend_wait` | 上涨趋势等待 | sell_trend_days |
| `guard.profit_ratio` | 利润比例保护 | profit_ratio_multiplier |
| `action.pause_auto_sell` | 暂停自动出售 |  |
| `action.steam_list` | Steam 上架 |  |

### app/sync_sold.py

SHA-256：`e07b7c8c640d4cb9b43a0aad023a95dea1326e6577c749635be2beb17a23e3db`

符号：`_fill_assetid_from_inventory:4`、`run_sync_sold_from_history:29`

依赖：`app.config_loader`、`app.inventory_cs2`、`app.state`、`app.steam_listings`、`app.trade_ledger`、`time`、`typing`

### app/trade_ledger.py

SHA-256：`07646afb9f87c48de50473b44c7983be15fbb81dae06a4937e8cdad97ecfe151`

符号：`_number:14`、`_rounded:22`、`_tradable_at:26`、`_stage:36`、`_ledger_item:48`、`build_trade_ledger:127`、`inventory_unlock_metadata:208`、`sync_unlocks_from_inventory:227`

依赖：`__future__`、`collections`、`datetime`、`time`、`typing`

### buff/__init__.py

SHA-256：`a387a4d59c5714862a213065f194d622c245d5cd637e646a5cb62866a4125b92`

符号：包入口，无定义

依赖：`.buyer`

### buff/buyer.py

SHA-256：`9320386b7c372d6a0a62082de3aae91f5473bec45be8855c05b0c68ac1d2294b`

符号：`BuffAuthExpired:21`、`BuffVerificationRequired:24`、`_is_auth_error:27`、`_is_verification_required:37`、`_parse_cookies:61`、`_csrf:69`、`BuffBuyer:71`、`__init__:72`、`_make_request:86`、`check_wait_pay_orders:118`、`get_sell_orders:141`、`get_goods_steam_price_cny:165`、`get_and_buy:192`、`_execute_post_buy:238`、`_fetch_pay_url:283`、`lock_and_get_pay_url:316`、`_get_alipay_url:357`、`_get_wechat_pay_url:372`、`_fetch_wechat_url:383`、`_generate_qr_code:391`、`batch_buy_create:408`、`batch_buy_wx_qrcode:443`、`batch_buy_finalize:459`、`ask_seller_to_send:498`、`is_order_paid:525`、`cancel_order:548`、`get_order_detail:565`、`get_order_state:585`

依赖：`json`、`logging`、`qrcode`、`random`、`requests`、`time`、`typing`、`urllib3`、`utils.delay`、`uuid`

### config/__init__.py

SHA-256：`a0a1925a61266d11a86700f312179cda502ce91556d33cf9afdd41fd66fd19cc`

符号：`_load:7`、`get:20`、`get_steam:26`、`get_buff:28`、`get_all_credentials:30`、`save_credentials:32`、`_filter_steam_cookies:39`、`_steam_id_from_cookies:49`、`update_steam_credentials:61`、`update_buff_credentials:74`、`get_app_config_path:84`、`load_app_config:86`、`save_app_config:94`

依赖：`json`、`pathlib`、`typing`

### run.py

SHA-256：`84f6b41d5cdc66d79548f21bbfe98591d8f1f4bc3d9f4304bf042c2d9041e4cd`

符号：包入口，无定义

依赖：`app.main`、`pathlib`、`sys`

### steam/__init__.py

SHA-256：`0c734950669d5afb66ea141cfac8a0add3bee6bcefae0cdf6a68c8c060c25292`

符号：包入口，无定义

依赖：`.client`、`.inventory`、`.market`、`.session`

### steam/client.py

SHA-256：`4bd0b468b6d3a2b7d4bbf7caaa627f9dbf77f9598a4be1249a791d42e15e11c4`

符号：`_extract_line1:16`、`detect_currency:34`、`build_listing_url:64`、`market_hash_name_from_listing_url:67`、`_parse_cookie_str:78`、`fetch_history:86`

依赖：`json`、`os`、`requests`、`typing`、`urllib.parse`、`urllib3`

### steam/inventory.py

SHA-256：`1ac1ac4ab614dc6f631af870736da9fe55e3a5fe578efa3683538aeead7d9eb8`

符号：`SteamAuthExpired:2`、`fetch_inventory:9`、`check_session_validity:30`、`_fetch_full_context:38`、`fetch_cs2_inventory:109`、`_fetch_ctx:119`、`find_asset_by_name:151`

依赖：`concurrent.futures`、`time`、`typing`、`utils.delay`

### steam/market.py

SHA-256：`184a1dfe59c5af3aaedf7df554f2c454aaca94c8ff9c19c677775cb20889d468`

符号：`_parse_sell_response:4`、`list_item:13`、`list_item_by_name:35`

依赖：`.inventory`、`json`、`typing`

### steam/market_orders.py

SHA-256：`ff9a5d4a978981416cddd1efb0e4130b40f633be0a7c509272e6c89da9e1ab9a`

符号：`clear_caches:25`、`_format_request_error:107`、`_http_error_reason:113`、`_extract_item_nameid:122`、`_load_exchange_rates:129`、`_extract_ssr_render_context:152`、`_normalize_market_hash_name:162`、`_extract_orderbook_query_name:167`、`_extract_description_query_name:178`、`_extract_ssr_queries:189`、`_strip_html_text:202`、`_extract_description_tag:207`、`_extract_ssr_description_data:216`、`_infer_cs2_quality_filter_tag:236`、`_extract_exterior_name_from_description:246`、`_build_filtered_group_listing_url:261`、`_steam_cents_to_cny:293`、`_parse_compact_orders_cny:312`、`_compact_orderbook_data_to_cny:334`、`_extract_ssr_orderbook_cny:396`、`_fetch_ssr_sell_orders_cny:442`、`_fetch_action_orderbook_cny:499`、`get_item_nameid:564`、`fetch_item_orders_histogram:621`、`cents_to_yuan:710`、`_parse_order_graph:712`、`_parse_sell_order_graph:728`、`get_sell_orders_cny:730`、`_get_dynamic_thresholds:848`、`compute_smart_list_price:858`、`get_lowest_sell_price_cny:905`

依赖：`app.database`、`json`、`logging`、`math`、`pathlib`、`re`、`steam.client`、`threading`、`time`、`typing`、`urllib.parse`、`utils.delay`、`utils.money`、`utils.proxy_manager`

### steam/session.py

SHA-256：`9f6a8f7723176049efcd72866aaea7dac2ffa181794b2d3a849c3ccdcdb34d7c`

符号：`parse_cookies:9`、`create_market_session:17`

依赖：`requests`、`typing`、`urllib3`

### steamdt/__init__.py

SHA-256：`13c5ea51b67e13f96604ad82918f418a1a85613ddef040679f9f1013e3269968`

符号：包入口，无定义

依赖：`.fetcher`、`.models`

### steamdt/fetcher.py

SHA-256：`15f164c4ac97337ae137c004b99b978900f71c0e3ec6780e887fd23666e1c850`

符号：`_make_timestamp:20`、`fetch_steamdt_data:28`

依赖：`.models`、`.parser`、`json`、`random`、`time`、`typing`、`urllib.error`、`urllib.request`

### steamdt/models.py

SHA-256：`f2bb73b1a648ba4fdb96621330d7e0a2c915556e1f9960f24a3ddc6c1ea70e2f`

符号：`SteamDTQueryParams:6`、`to_payload:20`、`SteamDTRow:36`

依赖：`dataclasses`、`typing`

### steamdt/parser.py

SHA-256：`1e94e4fc5bdb514dd6449e016c253909f9042d65aa0d22d2a51f2bae5e07f402`

符号：`_find_platform:6`、`parse_response:13`

依赖：`.models`、`typing`

### tests/__init__.py

SHA-256：`7571cd0be9938043ac5f16661613e49e35e8bee378b7c5e1cb3de5184e23a27e`

符号：包入口，无定义

依赖：无 import

### tests/test_account_region_sync.py

SHA-256：`7ce4cadcc96ac14cae14649f64613d0bc9ca71ec3f6c5b11e3b7981e1a0c460b`

符号：`_ProxyManager:9`、`get_proxies_for_request:10`、`_Response:14`、`__init__:17`、`json:21`、`test_refresh_account_region_currency_success_marks_account_safe:25`、`test_refresh_account_region_currency_uses_settlement_currency_as_region_source:49`、`test_refresh_account_region_currency_can_fallback_to_wallet_country_for_unknown_currency:76`、`test_refresh_account_region_currency_marks_missing_currency_unsafe:102`、`test_refresh_account_region_currency_skips_missing_cookie_during_startup:123`、`test_refresh_account_region_currency_marks_missing_cookie_unsafe_when_strict:143`、`test_verify_account_triggers_region_currency_refresh:162`、`test_get_base_auth_status_strict_reads_store_user_config_country:183`、`test_get_wallet_balance_returns_wallet_country:206`、`test_get_wallet_balance_rejects_unknown_currency_id:225`

依赖：`app`、`app.routes`、`app.services`、`html`、`json`、`pathlib`、`sys`、`utils.proxy_manager`

### tests/test_auth_relogin_state.py

SHA-256：`7ca0a7a846f4fd3eea86978eecd7c1ce63b22ff8a86ae358e9ee89763a2e1d43`

符号：`test_manual_buff_cookie_requires_session:6`、`test_relogin_finish_surfaces_worker_error:19`、`test_buff_auto_relogin_success_clears_auth_and_verification:37`、`FakePage:42`、`goto:43`、`wait_for_timeout:46`、`FakeContext:49`、`cookies:52`、`close:58`、`FakeChromium:61`、`launch_persistent_context:62`、`FakePlaywright:65`、`__enter__:66`、`__exit__:69`、`test_relogin_context_retries_with_temp_profile:93`、`FakeChromium:98`、`launch_persistent_context:99`、`test_browser_launch_error_is_user_friendly:119`

依赖：`app.routes`、`app.services`、`sys`、`threading`、`types`

### tests/test_background_startup.py

SHA-256：`b47e1b8c4a2bee8cc62b814faa35e366926a800693ee72ce13911096e53c7512`

符号：`test_task_queue_has_spare_capacity_for_startup_workers:7`、`test_region_sync_is_submitted_before_long_running_workers:16`、`FakeQueue:21`、`submit:22`、`test_wait_for_server_ready_probes_localhost_for_wildcard_host:35`、`FakeThread:40`、`is_alive:41`、`FakeSocket:44`、`__enter__:45`、`__exit__:48`、`fake_create_connection:51`、`test_wait_for_server_ready_stops_when_server_thread_exits:61`、`DeadThread:64`、`is_alive:65`、`fail_if_called:68`、`test_region_sync_worker_does_not_sleep_before_missing_account_skip:76`

依赖：`app`、`app.services`、`pathlib`、`sys`

### tests/test_buff_cooldown_modes.py

SHA-256：`0f8ee4c374c3194ae4a5aea910ce7a2869ad970b182365455be085c366176f44`

符号：`FakeClient:10`、`__init__:11`、`get_sell_orders:16`、`test_冷却模式兼容旧布尔参数:21`、`test_旧策略参数在加载时迁移为三段式:27`、`test_仅冷却模式使用全部与无冷却卖单的集合差:43`、`test_仅冷却模式在无冷却查询异常时失败关闭:55`

依赖：`app.services.buff_client`、`app.strategy_engine`

### tests/test_buff_verification.py

SHA-256：`b500b39f40b591dd8a08ddce542614a5dffde815f6678afcb4782e84c19e3496`

符号：`test_buff_page_expired_is_verification_required:4`、`test_make_request_raises_verification_required:10`、`FakeResponse:13`、`json:17`

依赖：`buff.buyer`、`pytest`

### tests/test_candidate_report.py

SHA-256：`5cd65798cf64f96b30bf4eb62ae39773e858fd84cb2fc5cc12a379b17e167efd`

符号：`test_candidate_snapshot包含快卖价差和过期状态:6`

依赖：`app.candidate_report`、`time`

### tests/test_candidate_routes.py

SHA-256：`4b9578c114aec85f6dc5a5c83b876a967bd35d2809bace94ad5ca720b5b114cc`

符号：`test_人工购买必须使用匹配的预检令牌:4`、`FakeBuyer:25`、`get_sell_orders:26`、`fake_start:44`、`test_人工购买拒绝缺失预检令牌:76`

依赖：`app.routes`、`app.state`

### tests/test_config_loader_validation.py

SHA-256：`58e8e22e9dbad971ecea41efe1cd4637dee8b31c3d424ca12be412286f079fca`

符号：`test_load_app_config_validated_applies_range_validation:8`、`test_save_app_config_validated_applies_range_validation:22`

依赖：`app`、`pathlib`、`sys`、`warnings`

### tests/test_config_routes.py

SHA-256：`e99effbb8b113f93673a8cc3901d76ae0f26d711296feaddd5d38fc8bec6d39f`

符号：`test_full_import_uses_validated_config_save:7`

依赖：`app.routes`、`pathlib`、`sys`

### tests/test_config_schema.py

SHA-256：`eda1efcdec0786a87f66555b227ae162dd4d81feccb997f0f8087587d6eab147`

符号：`test_merge_顶层覆盖:12`、`test_merge_深层合并不影响未指定key:18`、`test_merge_非dict会整体覆盖:26`、`test_merge_不修改原始defaults:31`、`test_validate_字符串转float:39`、`test_validate_字符串转int:45`、`test_validate_iflow_min_price_zero_is_preserved:50`、`test_validate_缺少section用默认值:55`、`test_range_max_discount超出被限制:62`、`test_range_cv_threshold为零被限制:71`、`test_range_正常值不被修改:79`、`test_range_price_tolerance负数被限制:90`

依赖：`app.config_schema`、`pathlib`、`sys`、`warnings`

### tests/test_config_schema_hardening.py

SHA-256：`a741d99b85c4bbadfbf4fee59605e443ba951b99d2796701716e48a9e7c23733`

符号：`test_merge_deepcopies_nested_mutable_defaults:9`、`test_validate_bool_strings_use_boolean_semantics:16`

依赖：`app.config_schema`、`pathlib`、`sys`

### tests/test_currency_conversion.py

SHA-256：`8492fa60416c8247c81efaebbba1f81b899f59df78c184d46c4d06e8fe63a520`

符号：`test_detect_currency_supports_steam_symbol_prefixes:9`、`test_apply_currency_uses_rate_map_for_non_usd_prices:24`、`test_stability_currency_conversion_uses_exchange_rate_map:33`

依赖：`analysis`、`pathlib`、`pytest`、`steam.client`、`sys`、`utils.money`

### tests/test_database.py

SHA-256：`bd4b87d8b6d1ed39f12d56c28af9835c3d76a5752a8a5c8211b091bb8bb86f17`

符号：`test_wilson_score_无评价返回零:12`、`test_wilson_score_基本正确性:18`、`test_wilson_score_少量样本被惩罚:28`、`test_wilson_score_结果在01之间:35`

依赖：`app.database`、`pathlib`、`sys`

### tests/test_holdings_stats.py

SHA-256：`a70077fd5dbdcce157b61b66995176484d9ec7a1d61bbc8ab93b7d8b901eb522`

符号：`test_total_market_change_only_counts_items_with_both_market_prices:4`

依赖：`app.notify`

### tests/test_market_orders.py

SHA-256：`303d3ce6dd4aec6dad873b05203020a2c3749b087938de41e386e9878b81bfd3`

符号：`test_新版订单簿支持INR转CNY:7`、`test_新版订单簿同时解析最高买单和买盘:28`、`test_旧版订单图买盘按价格倒序解析:46`、`test_新版订单簿缺少非CNY汇率会报清晰原因:54`、`test_汇率文件里的Steam市场币种都有ECurrency映射:74`、`test_汇率文件包含的非Steam本地市场币种不会误映射:86`、`test_新版订单簿按querykey精确匹配请求的market_hash_name:95`、`test_get_sell_orders_cny_优先使用新版ssr而不是旧item_nameid缓存:141`、`test_get_sell_orders_cny_聚合页会回退到带筛选的分组页面:162`、`_make_html:167`、`DummyResponse:217`、`__init__:218`、`DummySession:222`、`get:223`、`test_构造分组筛选页优先使用描述里的内部tag:244`、`test_构造分组筛选页能识别星号stattrak品质:287`、`test_新版_orderbook_action_按精确名称拉取目标变体:294`、`DummyResponse:302`、`json:305`、`DummySession:315`、`get:316`、`test_get_sell_orders_cny_ssr预取错变体时回退新版_orderbook_action:343`

依赖：`json`、`pathlib`、`steam`、`sys`

### tests/test_pipeline_lock.py

SHA-256：`915f2679359bccb0c169dbdd516e9a2e9311058daae40cdc75ea26109c9f48b5`

符号：`test_start_pipeline_rejects_duplicate_running_pipeline:4`、`fake_run_pipeline:10`、`test_start_pipeline_allows_restart_after_thread_exits:33`、`fake_run_pipeline:38`

依赖：`app`、`threading`

### tests/test_pipeline_steps.py

SHA-256：`6e29ddc83e74054792a4b27243b4134b91d622116179e442b68ff9aa85e9ae2d`

符号：`_行:10`、`test_卖压_空订单返回None:35`、`test_卖压_日销量为零返回None:39`、`test_卖压_大量挂单卖压高:44`、`test_卖压_少量挂单卖压低:52`、`test_卖压_价格断层识别:59`、`test_过滤_正常行通过:72`、`test_过滤_关键词命中被去掉:78`、`test_过滤_价格非正被去掉:89`、`test_过滤_非buff链接被去掉:100`、`test_过滤_topN限制数量:110`、`test_过滤_goods_id从url解析:117`

依赖：`app.pipeline_steps`、`pathlib`、`sys`、`types`

### tests/test_proxy_manager.py

SHA-256：`e9008fd56db68ea3164c887b6fa2849b2925f443587c725768899b4c4a3639ca`

符号：`_构造代理管理器:10`、`test_代理_score为0的永远不被选:27`、`test_代理_空池返回None:39`、`test_代理_全挂了返回None:44`、`test_代理_高分代理被选得明显更多:54`、`test_代理池关闭时不预热:68`

依赖：`collections`、`pathlib`、`sys`、`threading`、`unittest.mock`、`utils.proxy_manager`

### tests/test_runtime_env.py

SHA-256：`5f93c0d08c716e7e8745f018f1b2bc198f737f4cdb7761ba4f746ce0a1542a29`

符号：`test_server_requirements_omit_desktop_webview_dependency:12`、`test_legacy_app_main_exposes_fastapi_app_lazily:19`、`test_auto_linux_without_display_uses_server_mode:25`、`test_no_display_stays_effectively_headless_even_if_env_disables_headless:34`、`test_auto_linux_with_display_uses_desktop_mode:42`、`test_env_overrides_host_port_and_local_url:50`、`test_relogin_start_requires_manual_cookie_without_gui:60`、`fail_thread:76`

依赖：`app.main`、`app.routes`、`app.runtime_env`、`pathlib`

### tests/test_sell_pipeline_currency_guard.py

SHA-256：`5e827d89ed93f970c198f82461d6995838304a636ad521a660289adb31ca0565`

符号：`_make_sellable:13`、`_build_ctx:24`、`_log:30`、`test_missing_rate_aborts_listing_plan:39`、`test_valid_rate_converts_price:75`、`test_no_account_skips_sell_phase:118`、`test_missing_currency_code_aborts_sell_phase:141`、`_log:147`、`test_sell_phase_uses_realtime_currency_instead_of_stale_cache:169`、`_log:175`、`test_sell_phase_does_not_require_region_when_currency_is_confirmed:199`

依赖：`app.sell_pipeline`、`pytest`、`unittest.mock`、`utils.money`

### tests/test_static_routes.py

SHA-256：`1455e854e50dd6cd47e650998fc6b7f7777ec72699d8e485ec666cb03df77a79`

符号：`test_static_route_serves_files_inside_web_root:7`、`test_static_route_blocks_path_traversal:21`

依赖：`app.routes`、`pathlib`、`sys`

### tests/test_steam_auth_errors.py

SHA-256：`6d652f673978adc44769514e8304b3eea3a4585e7f3f57bb2e4ca0e12f517e4c`

符号：`test_steam_login_connection_error_message_is_explicit:7`

依赖：`app.services`、`pathlib`、`sys`

### tests/test_strategies.py

SHA-256：`af0ff1416c239d260b857c4e63688df4f3e10fc4f956a1710266b811a705cf2d`

符号：`test_legacy_sell_strategy_maps_to_system_presets:4`、`test_default_buy_strategy_contains_current_protection_chain:12`、`test_default_sell_strategy_uses_split_pricing_modules:36`、`test_activation_rejects_enabled_user_module:45`、`test_import_sanitizes_sensitive_fields_and_does_not_override_system_id:69`、`test_simulate_marks_trade_actions_as_dry_run:85`、`test_validation_rejects_duplicate_conflict_dependency_and_step_limit:105`、`test_split_sell_pricing_without_offset_resets_offset:151`、`test_sell_listing_requires_one_pricing_core:179`、`test_one_piece_sell_pricing_core_can_replace_split_pricing:195`、`test_declarative_user_module_can_activate_and_simulate:224`、`test_declarative_user_module_can_depend_on_one_piece_pricing:260`、`test_declarative_runtime_guard_blocks_at_stage:292`、`test_declarative_user_module_dependency_order_is_enforced:336`、`test_external_code_user_module_still_cannot_activate:364`、`test_step_param_validation_rejects_out_of_range_value:391`、`test_system_strategy_params_can_be_saved_to_config:408`、`fake_load:415`、`fake_save:418`、`test_system_strategy_params_can_restore_schema_defaults:437`、`fake_load:444`、`fake_save:447`、`test_system_strategy_save_rejects_structure_changes:470`

依赖：`app`

### tests/test_strategy_hardening.py

SHA-256：`711707c7fa438377d8ccf69a3253b38a1a4a1947f0d520cd35f8fe01d52f6baa`

符号：`test_strategy_import_enabled_string_false_stays_disabled:9`

依赖：`app`、`pathlib`、`sys`

### tests/test_trade_ledger.py

SHA-256：`2f441a22169de940cc590e32763eef3bda929da080ca4546680dd486dd21cf5e`

符号：`test_旧记录使用明确标注的七天估算解锁时间:10`、`test_已售记录计算真实钱包价值与预测误差:23`、`test_同日解锁集中度标记高风险:54`、`test_Steam库存冷却时间覆盖估算值:70`、`test_数据库字典转换保留交易审计字段:90`、`test_库存元数据在已可交易时使用观测时间:112`、`test_已记录的Steam真实解锁时间不会退化为观测时间:119`

依赖：`app.database`、`app.trade_ledger`

### utils/__init__.py

SHA-256：`7eb70257593da06f682a3ddda54a9d260d4fc514f645237f5ca74b08f8da61a6`

符号：包入口，无定义

依赖：无 import

### utils/delay.py

SHA-256：`5bf70029ca35c24294a245ea4a930a9dbe8d18420b95d2134595c9c4982e8a2e`

符号：`jittered_sleep:3`

依赖：`app.state`、`random`、`time`

### utils/money.py

SHA-256：`8185973f15a7218f10b204b5ddb4684d2bfd7674280d47f56e506a690a86942e`

符号：`calculate_fee:32`、`get_total_with_fees:38`、`to_valid_market_price:45`、`get_item_price_from_total:58`、`usd_to_cny:81`、`_load_exchange_rates:83`、`apply_currency:105`、`yuan_to_cents:122`、`list_price_display_to_cents:126`、`cents_to_yuan:136`

依赖：`json`、`math`、`pathlib`、`time`、`typing`

### utils/network_check.py

SHA-256：`147c60fe7531c9c83732704e3bf078020f9f9ff29980c26573d5777207a7ffad`

符号：`_ping_baidu:13`、`NetworkChecker:24`、`__init__:25`、`report_success:39`、`report_failure:43`、`wait_until_online:72`、`is_offline:97`、`_log:101`、`get_network_checker:113`

依赖：`requests`、`threading`、`time`、`typing`

### utils/proxy_manager.py

SHA-256：`20f5de9bccc429e8e8f6d5449cbfe6e629afd91f295190858267109210b9828b`

符号：`_load_proxy_pool_cfg:14`、`_build_proxy_url:21`、`_pm_log:29`、`test_one_proxy:35`、`ProxyManager:63`、`__init__:64`、`_reload:79`、`_sync_cycle:86`、`reload:93`、`warmup:106`、`get_strategy:155`、`is_proxy_enabled:157`、`get_next_proxy_dict:159`、`should_use_proxy_on_failure:172`、`should_always_use_proxy:175`、`get_proxies_for_request:178`、`get_proxy_manager:222`

依赖：`app.config_loader`、`app.state`、`concurrent.futures`、`random`、`requests`、`threading`、`time`、`typing`

### utils/throttle.py

SHA-256：`761895fc8f12efc3af01c749a3728aa13b0056cc89c36fdc4323a174cc755cd4`

符号：`DomainThrottle:5`、`__init__:6`、`wait:9`、`get_throttle:19`

依赖：`threading`、`time`、`typing`、`utils.delay`

### utils/time.py

SHA-256：`d7fff4a720f4b53657cf4eacf412d24eeea0df9eba35a33a023aa3b3e067fb7c`

符号：`parse_steam_history_date:5`、`parse_steam_cooldown:11`、`cutoff_days_ago:17`、`utc_timestamp:19`

依赖：`datetime`、`typing`

### utils/trend.py

SHA-256：`4d28b1438900029e067e1c5fe9a8e1041f5ada22e7586c28ae8e7ac317cd1e07`

符号：`_dynamic_sensitivity:2`、`calculate_trend_robust:8`

依赖：`numpy`

## 两版差异索引

相同路径不代表内容相同。非 Python 文件仅列结构差异；以下逐一比较 Python、JS、CSS、HTML 文件。

| 路径 | 相对 reference 的 audit 状态 |
| --- | --- |
| `analysis/__init__.py` | 相同 |
| `analysis/stability.py` | 不同，需合并行为 |
| `app/__init__.py` | 相同 |
| `app/__main__.py` | 相同 |
| `app/accounts.py` | 相同 |
| `app/api.py` | 相同 |
| `app/candidate_report.py` | audit 独有 |
| `app/config_loader.py` | 不同，需合并行为 |
| `app/config_schema.py` | 不同，需合并行为 |
| `app/database.py` | 不同，需合并行为 |
| `app/gift_engine.py` | 不同，需合并行为 |
| `app/inventory_cs2.py` | 相同 |
| `app/main.py` | 相同 |
| `app/notify.py` | 不同，需合并行为 |
| `app/pipeline.py` | 不同，需合并行为 |
| `app/pipeline_context.py` | 相同 |
| `app/pipeline_steps.py` | 不同，需合并行为 |
| `app/purchase_limit.py` | 相同 |
| `app/receive_flow.py` | 不同，需合并行为 |
| `app/repair_error_records.py` | 不同，需合并行为 |
| `app/routes/__init__.py` | 不同，需合并行为 |
| `app/routes/accounts.py` | 相同 |
| `app/routes/auth.py` | 不同，需合并行为 |
| `app/routes/candidates.py` | audit 独有 |
| `app/routes/config.py` | 不同，需合并行为 |
| `app/routes/gift.py` | 不同，需合并行为 |
| `app/routes/inventory.py` | 不同，需合并行为 |
| `app/routes/pipeline.py` | 不同，需合并行为 |
| `app/routes/proxy.py` | 相同 |
| `app/routes/static.py` | 不同，需合并行为 |
| `app/routes/status.py` | 不同，需合并行为 |
| `app/routes/steam_deals.py` | 不同，需合并行为 |
| `app/routes/strategies.py` | 相同 |
| `app/routes/transactions.py` | 不同，需合并行为 |
| `app/runtime_env.py` | 相同 |
| `app/sell_pipeline.py` | 不同，需合并行为 |
| `app/services/__init__.py` | 相同 |
| `app/services/account_region.py` | 不同，需合并行为 |
| `app/services/analysis_client.py` | 相同 |
| `app/services/buff_auth.py` | 不同，需合并行为 |
| `app/services/buff_checkout_guard.py` | reference 独有 |
| `app/services/buff_client.py` | 不同，需合并行为 |
| `app/services/deal_cards.py` | 不同，需合并行为 |
| `app/services/iflow_client.py` | 不同，需合并行为 |
| `app/services/retry.py` | 相同 |
| `app/services/steam_auth.py` | 不同，需合并行为 |
| `app/services/steam_client.py` | 不同，需合并行为 |
| `app/services/steam_deals.py` | 不同，需合并行为 |
| `app/services/task_queue.py` | 相同 |
| `app/services/workers.py` | 不同，需合并行为 |
| `app/shared_market.py` | 相同 |
| `app/state.py` | 不同，需合并行为 |
| `app/steam_confirm.py` | 相同 |
| `app/steam_delist.py` | 不同，需合并行为 |
| `app/steam_listings.py` | 相同 |
| `app/strategy_engine.py` | 不同，需合并行为 |
| `app/sync_sold.py` | 不同，需合并行为 |
| `app/trade_ledger.py` | audit 独有 |
| `buff/__init__.py` | 不同，需合并行为 |
| `buff/buyer.py` | 不同，需合并行为 |
| `buff/request_policy.py` | reference 独有 |
| `config/__init__.py` | 不同，需合并行为 |
| `run.py` | 相同 |
| `steam/__init__.py` | 不同，需合并行为 |
| `steam/client.py` | 不同，需合并行为 |
| `steam/inventory.py` | 相同 |
| `steam/market.py` | 不同，需合并行为 |
| `steam/market_orders.py` | 不同，需合并行为 |
| `steam/session.py` | 相同 |
| `steamdt/__init__.py` | 相同 |
| `steamdt/fetcher.py` | 不同，需合并行为 |
| `steamdt/models.py` | 相同 |
| `steamdt/parser.py` | 相同 |
| `tests/__init__.py` | 相同 |
| `tests/test_account_region_sync.py` | 不同，需合并行为 |
| `tests/test_auth_relogin_state.py` | 不同，需合并行为 |
| `tests/test_background_startup.py` | 相同 |
| `tests/test_buff_ask_seller.py` | reference 独有 |
| `tests/test_buff_auth_hardening.py` | reference 独有 |
| `tests/test_buff_batch_integrity.py` | reference 独有 |
| `tests/test_buff_checkout_guard.py` | reference 独有 |
| `tests/test_buff_checkout_integration.py` | reference 独有 |
| `tests/test_buff_cooldown_modes.py` | audit 独有 |
| `tests/test_buff_purchase_flow_safety.py` | reference 独有 |
| `tests/test_buff_purchase_persistence.py` | reference 独有 |
| `tests/test_buff_receive_mapping_safety.py` | reference 独有 |
| `tests/test_buff_receive_session.py` | reference 独有 |
| `tests/test_buff_request_policy.py` | reference 独有 |
| `tests/test_buff_verification.py` | 不同，需合并行为 |
| `tests/test_candidate_report.py` | audit 独有 |
| `tests/test_candidate_routes.py` | audit 独有 |
| `tests/test_config_loader_validation.py` | 不同，需合并行为 |
| `tests/test_config_routes.py` | 不同，需合并行为 |
| `tests/test_config_schema.py` | 不同，需合并行为 |
| `tests/test_config_schema_hardening.py` | 相同 |
| `tests/test_currency_conversion.py` | 相同 |
| `tests/test_database.py` | 相同 |
| `tests/test_holdings_stats.py` | 相同 |
| `tests/test_market_orders.py` | 不同，需合并行为 |
| `tests/test_pipeline_lock.py` | 相同 |
| `tests/test_pipeline_steps.py` | 不同，需合并行为 |
| `tests/test_pipeline_time_window.py` | reference 独有 |
| `tests/test_proxy_manager.py` | 不同，需合并行为 |
| `tests/test_repair_error_records.py` | reference 独有 |
| `tests/test_runtime_env.py` | 相同 |
| `tests/test_sell_listing_response.py` | reference 独有 |
| `tests/test_sell_pipeline_currency_guard.py` | 相同 |
| `tests/test_sell_pipeline_ownership_guard.py` | reference 独有 |
| `tests/test_static_routes.py` | 不同，需合并行为 |
| `tests/test_status_time_export.py` | reference 独有 |
| `tests/test_steam_auth_errors.py` | 不同，需合并行为 |
| `tests/test_steam_client.py` | reference 独有 |
| `tests/test_steamdt_proxy.py` | reference 独有 |
| `tests/test_strategies.py` | 相同 |
| `tests/test_strategy_hardening.py` | 相同 |
| `tests/test_time_utils.py` | reference 独有 |
| `tests/test_trade_ledger.py` | audit 独有 |
| `tests/test_transaction_sale_price_semantics.py` | reference 独有 |
| `tests/test_transactions_delist.py` | reference 独有 |
| `utils/__init__.py` | 相同 |
| `utils/delay.py` | 相同 |
| `utils/money.py` | 相同 |
| `utils/network_check.py` | 相同 |
| `utils/proxy_manager.py` | 不同，需合并行为 |
| `utils/throttle.py` | 相同 |
| `utils/time.py` | 不同，需合并行为 |
| `utils/trend.py` | 相同 |
| `web/css/animations.css` | 相同 |
| `web/css/base.css` | 相同 |
| `web/css/components.css` | 不同，需合并行为 |
| `web/css/layout.css` | 相同 |
| `web/css/variables.css` | 相同 |
| `web/index.html` | 不同，需合并行为 |
| `web/js/accounts.js` | 不同，需合并行为 |
| `web/js/candidateDashboard.js` | audit 独有 |
| `web/js/console.js` | 相同 |
| `web/js/gift.js` | 相同 |
| `web/js/main.js` | 不同，需合并行为 |
| `web/js/proxy.js` | 相同 |
| `web/js/settings.js` | 不同，需合并行为 |
| `web/js/steamDeals.js` | 不同，需合并行为 |
| `web/js/steamGuard.js` | 相同 |
| `web/js/strategies.js` | 不同，需合并行为 |
| `web/js/theme.js` | 相同 |
| `web/js/tradeLedger.js` | audit 独有 |
| `web/js/transactions.js` | 不同，需合并行为 |
| `web/js/utils.js` | 不同，需合并行为 |
