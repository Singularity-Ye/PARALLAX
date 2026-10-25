use super::{AppError, accounts::Accounts, ports::*};
use crate::domain::{buff_orders::*, selection::now};
use std::sync::Arc;

pub struct BuffOrderResearch<R, C, V, P> {
    repo: R,
    accounts: Arc<Accounts<R, C, V>>,
    provider: P,
}
impl<
    R: AccountRepository + BuffOrderRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: BuffOrderProvider,
> BuffOrderResearch<R, C, V, P>
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
        id: &str,
        request: OrderHistoryRequest,
    ) -> Result<OrderHistorySnapshot, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        let mut access = self.accounts.buff_access(id).await?;
        let principal = access
            .session
            .view
            .subject
            .clone()
            .ok_or(AppError::Conflict("BUFF主体未知"))?;
        let started = now();
        let mut pages = Vec::new();
        let mut count = 0usize;
        for number in 1..=request.max_pages {
            // One bounded logical observation holds the same credential lease.
            // Persist pacing before each request, including a failed/crashed read.
            if number > 1 {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            access.pace().await?;
            let result = self
                .provider
                .history_page(number, request.page_size, &principal, &access.secret)
                .await
                .map_err(AppError::Buff);
            let page = access.finish(result).await?;
            page.validate(number, request.page_size)
                .map_err(|_| AppError::Buff(BuffReadError::InvalidPayload))?;
            count += page.orders.len();
            let end = page.orders.len() < request.page_size as usize
                || page.reported_total == Some(count as u64);
            pages.push(page);
            if end {
                break;
            }
        }
        let snapshot = OrderHistorySnapshot::new(
            id.into(),
            principal,
            access.session.view.generation.clone(),
            request,
            pages,
            started,
            now(),
        )
        .map_err(AppError::Invalid)?;
        // The auth lease remains held through the immutable insert.
        if !self.repo.insert_buff_orders(snapshot.clone()).await? {
            return Err(AppError::Conflict(
                "读取期间BUFF身份或凭据代次发生变化，未保存新观察",
            ));
        }
        Ok(snapshot)
    }
    pub async fn list(&self, account: &str) -> Result<Vec<OrderHistorySnapshot>, AppError> {
        self.accounts.account(account).await?;
        Ok(self.repo.buff_order_observations(account.into()).await?)
    }
    pub async fn get(&self, account: &str, id: String) -> Result<OrderHistorySnapshot, AppError> {
        self.accounts.account(account).await?;
        self.repo
            .buff_orders(id)
            .await?
            .filter(|s| s.account_id == account)
            .ok_or(AppError::Missing("账户订单观察不存在"))
    }
    pub async fn lookup(
        &self,
        account: &str,
        id: String,
        request: OrderLookupRequest,
    ) -> Result<OrderLookup, AppError> {
        self.get(account, id)
            .await?
            .lookup(&request.order_id, now())
            .map_err(AppError::Invalid)
    }
}
#[cfg(test)]
#[path = "buff_orders_tests.rs"]
mod tests;
