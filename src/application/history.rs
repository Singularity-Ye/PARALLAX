use super::{AppError, accounts::Accounts, ports::*};
use crate::domain::{
    history::*,
    selection::{SourceKind, now},
};
use std::sync::Arc;
pub struct HistoryResearch<R, C, V, P> {
    repo: R,
    accounts: Arc<Accounts<R, C, V>>,
    provider: P,
}
impl<
    R: AccountRepository + Repository + HistoryRepository + InventoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: HistoryProvider,
> HistoryResearch<R, C, V, P>
{
    pub fn new(repo: R, accounts: Arc<Accounts<R, C, V>>, provider: P) -> Arc<Self> {
        Arc::new(Self {
            repo,
            accounts,
            provider,
        })
    }
    pub async fn fetch(
        &self,
        account: &str,
        request: HistoryRequest,
    ) -> Result<HistorySnapshot, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        self.accounts.account(account).await?;
        let identity_binding = if let Some(id) = &request.inventory_snapshot_id {
            let inventory = self
                .repo
                .inventory(id.clone())
                .await?
                .filter(|s| s.account_id == account)
                .ok_or(AppError::Missing("账户库存快照不存在"))?;
            if !inventory.assets.iter().any(|a| {
                a.reported_marketable && a.market_hash_name.as_deref() == Some(&request.item_key)
            }) {
                return Err(AppError::Missing("库存快照中没有此可研究的市场商品"));
            }
            "inventory_snapshot_and_request_parameters"
        } else {
            let scan = self
                .repo
                .get(request.scan_id.clone())
                .await?
                .ok_or(AppError::Missing("扫描不存在"))?;
            if scan.status != "completed" || scan.request.source != SourceKind::Steamdt {
                return Err(AppError::Conflict(
                    "历史读取只接受已完成的真实 SteamDT 候选；演示数据禁止请求平台",
                ));
            }
            if !scan
                .candidates
                .iter()
                .any(|c| c.item.key == request.item_key)
            {
                return Err(AppError::Missing("扫描候选不存在"));
            }
            "scan_candidate_and_request_parameters"
        };
        let mut access = self
            .accounts
            .steam_access(account, SteamReadScope::PriceHistory)
            .await?;
        let at = now();
        self.repo
            .set_steam_read_retry(SteamReadScope::PriceHistory, at + 2)
            .await?;
        let result = self
            .provider
            .history(&request, &access.secret, at)
            .await
            .map_err(AppError::SteamRead);
        let series = access.finish(result).await?;
        let analysis = analyze(&series, &request, at).map_err(AppError::Invalid)?;
        let snapshot = HistorySnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: account.into(),
            credential_generation: access.session.view.generation.clone(),
            request,
            started_at: at,
            observed_at: now(),
            source: "steam_authenticated_pricehistory".into(),
            identity_binding: identity_binding.into(),
            platform_verified: false,
            series,
            analysis,
        };
        // Keep credential replacement serialized until this immutable evidence is committed.
        self.repo.insert_history(snapshot.clone()).await?;
        Ok(snapshot)
    }
    pub async fn list(&self, account: &str) -> Result<Vec<HistorySnapshot>, AppError> {
        self.accounts.account(account).await?;
        Ok(self.repo.histories(account.into()).await?)
    }
    pub async fn get(&self, account: &str, id: String) -> Result<HistorySnapshot, AppError> {
        self.accounts.account(account).await?;
        self.repo
            .history(id)
            .await?
            .filter(|s| s.account_id == account)
            .ok_or(AppError::Missing("账户价格历史快照不存在"))
    }
}
#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
