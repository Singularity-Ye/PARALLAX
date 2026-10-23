use super::{AppError, accounts::Accounts, ports::*};
use crate::domain::{accounts::Credentials, inventory::*, selection::now};
use std::{collections::HashSet, sync::Arc};

#[cfg(test)]
#[path = "inventory_tests.rs"]
mod tests;

pub struct InventoryResearch<R, C, V, P> {
    repo: R,
    accounts: Arc<Accounts<R, C, V>>,
    provider: P,
}
impl<
    R: AccountRepository + InventoryRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: InventoryProvider,
> InventoryResearch<R, C, V, P>
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
        request: InventoryRequest,
    ) -> Result<InventorySnapshot, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let snapshot = self
            .accounts
            .read_inventory(account, request, &self.provider)
            .await?;
        self.repo.insert_inventory(snapshot.clone()).await?;
        Ok(snapshot)
    }
    pub async fn list(&self, account: &str) -> Result<Vec<InventorySnapshot>, AppError> {
        self.accounts.account(account).await?;
        Ok(self.repo.inventories(account.into()).await?)
    }
    pub async fn get(&self, account: &str, id: String) -> Result<InventorySnapshot, AppError> {
        self.accounts.account(account).await?;
        self.repo
            .inventory(id)
            .await?
            .filter(|s| s.account_id == account)
            .ok_or(AppError::Missing("账户库存快照不存在"))
    }
}

// Called under the shared account gate for the entire two-context read. Secrets never enter snapshots.
pub(super) async fn collect<P: InventoryProvider, R: SteamReadPolicy>(
    provider: &P,
    repo: &R,
    steam_id: &str,
    secret: &Credentials,
    request: &InventoryRequest,
) -> Result<(Vec<InventoryAsset>, Vec<InventoryCoverage>), AppError> {
    use SteamReadError as E;
    let mut assets = Vec::new();
    let mut coverage = Vec::new();
    let mut seen_assets = HashSet::new();
    let mut last_request_at = None;
    for context in ["2", "16"] {
        let mut cursor = None;
        let mut seen_cursors = HashSet::new();
        let mut expected_total = None;
        let mut observed = 0;
        for page_number in 1..=request.max_pages_per_context {
            // Pacing between every request, including the context boundary; no automatic retries.
            if let Some(at) = last_request_at {
                tokio::time::sleep_until(at + std::time::Duration::from_secs(1)).await;
            }
            repo.set_steam_read_retry(SteamReadScope::Inventory, now() + 2)
                .await?;
            last_request_at = Some(tokio::time::Instant::now());
            let page = provider
                .page(steam_id, context, cursor.as_deref(), secret)
                .await
                .map_err(AppError::SteamRead)?;
            page.validate(context)
                .map_err(|_| AppError::SteamRead(E::InvalidPayload))?;
            if expected_total.is_some_and(|n| n != page.total) {
                return Err(AppError::SteamRead(E::ChangedDuringRead));
            }
            expected_total = Some(page.total);
            // A duplicate can be pagination drift or a context move. Never silently double count.
            if page
                .assets
                .iter()
                .any(|a| !seen_assets.insert(a.asset_id.clone()))
            {
                return Err(AppError::SteamRead(E::ChangedDuringRead));
            }
            observed += page.assets.len() as u32;
            if observed > page.total
                || (page.next_asset_id.is_none() && observed != page.total)
                || (page.next_asset_id.is_some() && observed >= page.total)
            {
                return Err(AppError::SteamRead(E::ChangedDuringRead));
            }
            assets.extend(page.assets);
            let exhausted = page.next_asset_id.is_none();
            if let Some(next) = &page.next_asset_id
                && (!seen_cursors.insert(next.clone()) || cursor.as_ref() == Some(next))
            {
                return Err(AppError::SteamRead(E::ChangedDuringRead));
            }
            if exhausted || page_number == request.max_pages_per_context {
                coverage.push(InventoryCoverage {
                    context_id: context.into(),
                    pages_read: page_number,
                    reported_total: page.total,
                    observed_assets: observed,
                    exhausted,
                    stop_reason: if exhausted {
                        "source_exhausted"
                    } else {
                        "page_budget"
                    }
                    .into(),
                });
                break;
            }
            cursor = page.next_asset_id;
        }
    }
    Ok((assets, coverage))
}
