use super::{AppError, accounts::Accounts, ports::*};
use crate::domain::{
    buff_quotes::*,
    selection::{SourceKind, now},
};
use std::sync::Arc;

#[cfg(test)]
#[path = "buff_quotes_tests.rs"]
mod tests;

pub struct BuffResearch<R, C, V, P> {
    repo: R,
    accounts: Arc<Accounts<R, C, V>>,
    provider: P,
}
impl<
    R: AccountRepository + Repository + BuffQuoteRepository,
    C: CredentialStore,
    V: SessionVerifier,
    P: BuffQuoteProvider,
> BuffResearch<R, C, V, P>
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
        request: BuffQuoteRequest,
    ) -> Result<BuffQuoteSnapshot, AppError> {
        request.validate().map_err(AppError::Invalid)?;
        self.accounts.account(id).await?;
        let scan = self
            .repo
            .get(request.scan_id.clone())
            .await?
            .ok_or(AppError::Missing("扫描不存在"))?;
        if scan.status != "completed" || scan.request.source != SourceKind::Steamdt {
            return Err(AppError::Conflict(
                "只能读取已完成的真实 SteamDT 扫描候选，演示数据不能请求 BUFF",
            ));
        }
        let item = &scan
            .candidates
            .iter()
            .find(|c| c.item.key == request.item_key)
            .ok_or(AppError::Missing("扫描候选不存在"))?
            .item;
        let goods_id = item
            .goods_id
            .filter(|id| *id > 0)
            .ok_or(AppError::Invalid("候选缺少 BUFF goods_id".into()))?;
        let (page, generation) = self
            .accounts
            .read_buff(id, goods_id, &request.item_key, &self.provider)
            .await?;
        let quote = BuffQuoteSnapshot::new(id.into(), generation, request, page, now())
            .map_err(AppError::Invalid)?;
        self.repo.insert_buff_quote(quote.clone()).await?;
        Ok(quote)
    }
    pub async fn get(&self, account: &str, id: String) -> Result<BuffQuoteSnapshot, AppError> {
        self.accounts.account(account).await?;
        self.repo
            .buff_quote(id)
            .await?
            .filter(|q| q.account_id == account)
            .ok_or(AppError::Missing("账户报价快照不存在"))
    }
    pub async fn list(&self, account: &str) -> Result<Vec<BuffQuoteSnapshot>, AppError> {
        self.accounts.account(account).await?;
        Ok(self.repo.buff_quotes(account.into()).await?)
    }
}
