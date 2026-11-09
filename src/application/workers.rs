use super::{
    accounts::Accounts,
    fx::FxResolver,
    ledger::{LedgerStore, PurchaseLedger},
    notify::NotifyService,
    ports::{CredentialStore, GuardSecretStore, SessionVerifier},
    steam_guard::SteamGuard,
};
use crate::domain::{
    ledger::{LotAction, LotEventRequest},
    selection::now,
    workers::{SchedulerSnapshot, WorkerInfo, WorkerState},
};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

struct WorkerStateInternal {
    info: WorkerInfo,
    trigger_requested: bool,
}

pub struct BackgroundWorkers<R, C, SV, GV> {
    accounts: Arc<Accounts<R, C, SV>>,
    guard: Arc<SteamGuard<R, GV>>,
    ledger: Arc<PurchaseLedger<R>>,
    fx: Option<Arc<dyn FxResolver>>,
    notify: Arc<NotifyService>,
    workers: Mutex<HashMap<String, WorkerStateInternal>>,
    running_gate: Mutex<()>,
}

impl<R, C, SV, GV> BackgroundWorkers<R, C, SV, GV>
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    pub fn new(
        accounts: Arc<Accounts<R, C, SV>>,
        guard: Arc<SteamGuard<R, GV>>,
        ledger: Arc<PurchaseLedger<R>>,
        fx: Option<Arc<dyn FxResolver>>,
        notify: Arc<NotifyService>,
    ) -> Arc<Self> {
        let mut map = HashMap::new();
        map.insert(
            "listing_reconciliation".into(),
            WorkerStateInternal {
                info: WorkerInfo {
                    name: "listing_reconciliation".into(),
                    display_name: "在售状态巡检与自动结售".into(),
                    description: "定时拉取 Steam 真实在售列表，已售出饰品自动结算台账并推送微信，待确认挂单自动批准".into(),
                    interval_secs: 600,
                    state: WorkerState::Idle,
                    last_run_at: None,
                    next_run_at: Some(now() + 60),
                    run_count: 0,
                    success_count: 0,
                    failure_count: 0,
                    last_message: Some("就绪，等待首次调度".into()),
                },
                trigger_requested: false,
            },
        );
        map.insert(
            "fx_refresh".into(),
            WorkerStateInternal {
                info: WorkerInfo {
                    name: "fx_refresh".into(),
                    display_name: "官方中间价汇率自动巡检".into(),
                    description: "定期从参考汇率服务拉取 17 国法币日参考汇率，保证 UAH/CNY/USD 换算不过期".into(),
                    interval_secs: 7200,
                    state: WorkerState::Idle,
                    last_run_at: None,
                    next_run_at: Some(now() + 180),
                    run_count: 0,
                    success_count: 0,
                    failure_count: 0,
                    last_message: Some("就绪，等待首次调度".into()),
                },
                trigger_requested: false,
            },
        );
        map.insert(
            "account_sync".into(),
            WorkerStateInternal {
                info: WorkerInfo {
                    name: "account_sync".into(),
                    display_name: "Steam 画像与钱包余额同步".into(),
                    description: "定期拉取 Steam 个人资料、头像、昵称与钱包最新可用余额和延迟余额".into(),
                    interval_secs: 3600,
                    state: WorkerState::Idle,
                    last_run_at: None,
                    next_run_at: Some(now() + 300),
                    run_count: 0,
                    success_count: 0,
                    failure_count: 0,
                    last_message: Some("就绪，等待首次调度".into()),
                },
                trigger_requested: false,
            },
        );

        let scheduler = Arc::new(Self {
            accounts,
            guard,
            ledger,
            fx,
            notify,
            workers: Mutex::new(map),
            running_gate: Mutex::new(()),
        });

        // Launch asynchronous worker supervisor
        let supervisor = scheduler.clone();
        tokio::spawn(async move {
            supervisor.run_loop().await;
        });

        scheduler
    }

    pub async fn snapshot(&self) -> SchedulerSnapshot {
        let workers_guard = self.workers.lock().await;
        let mut list: Vec<WorkerInfo> = workers_guard.values().map(|w| w.info.clone()).collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        SchedulerSnapshot {
            workers: list,
            scheduler_running: true,
            observed_at: now(),
        }
    }

    pub async fn trigger(&self, name: &str) -> anyhow::Result<()> {
        let mut workers_guard = self.workers.lock().await;
        let w = workers_guard
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("未知的 Worker 任务: {name}"))?;
        w.trigger_requested = true;
        w.info.next_run_at = Some(now());
        Ok(())
    }

    async fn run_loop(self: Arc<Self>) {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let to_run = {
                let current_time = now();
                let mut workers_guard = self.workers.lock().await;
                let mut candidates = Vec::new();
                for (name, internal) in workers_guard.iter_mut() {
                    let should_run = internal.trigger_requested
                        || internal
                            .info
                            .next_run_at
                            .is_some_and(|target| current_time >= target);
                    if should_run && internal.info.state != WorkerState::Running {
                        internal.trigger_requested = false;
                        internal.info.state = WorkerState::Running;
                        candidates.push(name.clone());
                    }
                }
                candidates
            };

            for worker_name in to_run {
                let this = self.clone();
                let w_name = worker_name.clone();
                tokio::spawn(async move {
                    let start_at = now();
                    let res = this.execute_worker(&w_name).await;
                    let finished_at = now();
                    let mut workers_guard = this.workers.lock().await;
                    if let Some(internal) = workers_guard.get_mut(&w_name) {
                        internal.info.state = WorkerState::Idle;
                        internal.info.last_run_at = Some(finished_at);
                        internal.info.next_run_at = Some(finished_at + internal.info.interval_secs);
                        internal.info.run_count += 1;
                        match res {
                            Ok(msg) => {
                                internal.info.success_count += 1;
                                internal.info.last_message = Some(format!(
                                    "✓ 成功（耗时 {}s）：{}",
                                    finished_at.saturating_sub(start_at),
                                    msg
                                ));
                            }
                            Err(err) => {
                                internal.info.failure_count += 1;
                                internal.info.last_message = Some(format!(
                                    "✕ 失败（耗时 {}s）：{}",
                                    finished_at.saturating_sub(start_at),
                                    err
                                ));
                            }
                        }
                    }
                });
            }
        }
    }

    async fn execute_worker(&self, name: &str) -> anyhow::Result<String> {
        let _single_flight = self.running_gate.lock().await;
        match name {
            "listing_reconciliation" => self.run_listing_reconciliation().await,
            "fx_refresh" => self.run_fx_refresh().await,
            "account_sync" => self.run_account_sync().await,
            _ => anyhow::bail!("未注册的任务类型: {name}"),
        }
    }

    async fn run_listing_reconciliation(&self) -> anyhow::Result<String> {
        let accounts = self.accounts.list().await?;
        if accounts.is_empty() {
            return Ok("无可用账户".into());
        }

        let mut total_active = 0;
        let mut total_confirmed = 0;
        let mut total_sold = 0;

        for acc in &accounts {
            let cred_res = self.accounts.steam_credentials(&acc.id).await;
            let (account, cred) = match cred_res {
                Ok(pair) => pair,
                Err(e) => {
                    self.notify
                        .notify_auth_expired(&acc.label, &e.to_string())
                        .await;
                    continue;
                }
            };

            // 1. Fetch live market listings
            let snapshot = match crate::adapters::steam_market::fetch_my_listings(&cred.cookies).await {
                Ok(s) => s,
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("400") || err_str.contains("401") || err_str.contains("403") {
                        self.notify
                            .notify_auth_expired(&acc.label, "Steam 登录已过期（获取在售列表受阻）")
                            .await;
                    }
                    continue;
                }
            };
            total_active += snapshot.active.len();

            // 2. Auto-confirm pending listings if identity_secret is available
            if !snapshot.to_confirm.is_empty() {
                if let Ok((_, id_sec, dev_id)) = self.guard.identity_credentials(&acc.id).await {
                    if let Ok(confs) = crate::adapters::steam_market::get_mobile_confirmations(
                        &id_sec,
                        &dev_id,
                        &account.steam_id,
                        &cred.cookies,
                    )
                    .await
                    {
                        if !confs.is_empty() {
                            if let Ok(n) = crate::adapters::steam_market::accept_mobile_confirmations(
                                &id_sec,
                                &dev_id,
                                &account.steam_id,
                                &cred.cookies,
                                &confs,
                            )
                            .await
                            {
                                total_confirmed += n;
                            }
                        }
                    }
                }
            }

            // 3. Reconcile ledger lots
            let lots = self.ledger.list(acc.id.clone()).await?;
            let active_asset_ids: std::collections::HashSet<String> = snapshot
                .active
                .iter()
                .chain(snapshot.to_confirm.iter())
                .map(|l| l.asset_id.clone())
                .collect();

            let now_sec = now();
            for lot in lots {
                let mut rev = lot.revision;
                for a in &lot.assets {
                    if a.stage == crate::domain::ledger::AssetStage::Listed {
                        // If the asset is no longer in active or pending listings, it was sold!
                        if !active_asset_ids.contains(&a.asset.asset_id) {
                            let listing_ref = a
                                .listing
                                .as_ref()
                                .map(|l| l.listing_ref.clone())
                                .unwrap_or_else(|| format!("wall_list_{}", a.asset.asset_id));
                            let gross_minor = a
                                .listing
                                .as_ref()
                                .map(|l| l.gross.minor)
                                .unwrap_or(a.cost_cents);
                            let net_minor = (gross_minor as f64 * 0.8696).round() as i64;
                            let currency = a
                                .listing
                                .as_ref()
                                .map(|l| l.gross.currency.clone())
                                .unwrap_or_else(crate::domain::money::Currency::cny);
                            let sale_ref = format!("auto_sold_{}_{}", a.asset.asset_id, now_sec);

                            let event_action = LotAction::Sell {
                                asset_id: a.asset.asset_id.clone(),
                                listing_ref,
                                sale_ref,
                                net_wallet: crate::domain::money::Money {
                                    minor: net_minor.max(0) as u64,
                                    currency: currency.clone(),
                                },
                                sold_at: now_sec,
                            };

                            let append_res = self
                                .ledger
                                .append(
                                    acc.id.clone(),
                                    lot.id.clone(),
                                    LotEventRequest {
                                        client_key: uuid::Uuid::new_v4().to_string(),
                                        expected_revision: rev,
                                        action: event_action,
                                    },
                                )
                                .await;

                            if let Ok(outcome) = append_res {
                                rev = outcome.lot.revision;
                                total_sold += 1;
                                let net_str = format!("{:.2} {currency}", net_minor as f64 / 100.0);
                                self.notify
                                    .notify_sold(
                                        &acc.label,
                                        &lot.request.item_key,
                                        &a.asset.asset_id,
                                        &net_str,
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
        }

        Ok(format!(
            "在售活跃 {} 件 · 自动免手机批准 {} 项 · 自动核销已售出入账 {} 件",
            total_active, total_confirmed, total_sold
        ))
    }

    async fn run_fx_refresh(&self) -> anyhow::Result<String> {
        if let Some(fx) = self.fx.as_ref() {
            let snap = fx.rates().await?;
            let uah_rate = snap
                .rates
                .get("UAH")
                .map(|r| format!("{:.4}", r.units_per_cny_1e8 as f64 / 1e8))
                .unwrap_or_else(|| "缺失".into());
            let usd_rate = snap
                .rates
                .get("USD")
                .map(|r| format!("{:.4}", r.units_per_cny_1e8 as f64 / 1e8))
                .unwrap_or_else(|| "缺失".into());
            Ok(format!(
                "已获取 {} 种法币日参考汇率（1 CNY = {} UAH, {} USD）",
                snap.rates.len(),
                uah_rate,
                usd_rate
            ))
        } else {
            Ok("未挂载外部汇率解析源".into())
        }
    }

    async fn run_account_sync(&self) -> anyhow::Result<String> {
        let accounts = self.accounts.list().await?;
        let mut synced = 0;
        for acc in &accounts {
            let cookies = self.accounts.stored_steam_cookies(&acc.id).await;
            #[cfg(windows)]
            {
                let profile = crate::adapters::steam_password::fetch_profile_and_wallet(
                    &acc.steam_id,
                    cookies.as_deref(),
                )
                .await;
                let mut current = self.accounts.profile_meta(&acc.id).await.unwrap_or_default();
                let mut changed = false;
                if !profile.display_name.is_empty() && current.display_name != profile.display_name {
                    current.display_name = profile.display_name;
                    changed = true;
                }
                if !profile.avatar_url.is_empty() && current.avatar_url != profile.avatar_url {
                    current.avatar_url = profile.avatar_url;
                    changed = true;
                }
                if !profile.currency_code.is_empty() {
                    current.currency_code = profile.currency_code;
                    current.region_code = profile.region_code;
                    current.wallet_balance_display = profile.wallet_balance_display;
                    current.wallet_balance_raw = profile.wallet_balance_raw;
                    current.wallet_currency_id = profile.wallet_currency_id;
                    changed = true;
                }
                if changed {
                    let _ = self.accounts.save_profile_meta(&acc.id, current).await;
                    synced += 1;
                }
            }
        }
        Ok(format!("巡检完成：成功同步更新 {} 个账户画像与钱包余额", synced))
    }
}
