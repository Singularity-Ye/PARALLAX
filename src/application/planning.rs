use super::ports::{MarketSource, Repository};
use super::{AppError, Application, ports};
use crate::domain::{
    planning::{PlanPreview, PlanRequest, preview_with_fx_quotes},
    selection::now,
};
impl<S: MarketSource + 'static, R: Repository + 'static> Application<S, R> {
    pub async fn create_plan(&self, request: PlanRequest) -> Result<PlanPreview, AppError>
    where
        R: ports::AccountRepository + ports::OrderBookRepository + ports::BuffQuoteRepository,
    {
        let plan = self.prepare_plan(request).await?;
        self.store.insert_plan(plan.clone()).await?;
        Ok(plan)
    }
    pub(super) async fn prepare_plan(&self, request: PlanRequest) -> Result<PlanPreview, AppError>
    where
        R: ports::AccountRepository + ports::OrderBookRepository + ports::BuffQuoteRepository,
    {
        request.validate().map_err(AppError::Invalid)?;
        if let Some(snapshot) = &request.position_snapshot {
            self.store
                .account(snapshot.account_id.clone())
                .await?
                .ok_or(AppError::Missing("仓位快照的账户不存在"))?;
        }
        let report = self.scan(request.scan_id.clone()).await?;
        let mut books = Vec::new();
        for id in &request.steam_snapshot_ids {
            books.push(
                self.store
                    .order_book(id.clone())
                    .await?
                    .ok_or(AppError::Missing("Steam 盘口快照不存在"))?,
            );
        }
        let mut buffs = Vec::new();
        if !request.buff_snapshot_ids.is_empty() {
            use crate::domain::accounts::{Platform, SessionStatus};
            let account = request.buff_account_id.as_ref().unwrap();
            self.store
                .account(account.clone())
                .await?
                .ok_or(AppError::Missing("BUFF 报价账户不存在"))?;
            let session = self
                .store
                .session(account.clone(), Platform::Buff)
                .await?
                .ok_or(AppError::Conflict("BUFF 会话不存在，请重新登录"))?;
            let at = now();
            if session.view.status != SessionStatus::Valid
                || session.view.subject.is_none()
                || session
                    .view
                    .checked_at
                    .is_none_or(|time| time > at || at - time > 300)
            {
                return Err(AppError::Conflict(
                    "BUFF 引用需要最近五分钟内核验有效的会话",
                ));
            }
            if self.store.buff_policy().await?.1 {
                return Err(AppError::Conflict("BUFF 验证挑战尚未解除"));
            }
            for id in &request.buff_snapshot_ids {
                let quote = self
                    .store
                    .buff_quote(id.clone())
                    .await?
                    .ok_or(AppError::Missing("BUFF 报价快照不存在"))?;
                if &quote.account_id != account
                    || quote.credential_generation != session.view.generation
                {
                    return Err(AppError::Conflict(
                        "BUFF 快照账户或凭据代次已变化，请重新读取",
                    ));
                }
                buffs.push(quote);
            }
        }
        let fx = if books
            .iter()
            .any(|b| b.book.currency != crate::domain::money::Currency::cny())
        {
            Some(
                self.fx
                    .as_ref()
                    .ok_or(AppError::Invalid("外币预算引用需要有效汇率来源".into()))?
                    .rates()
                    .await?,
            )
        } else {
            None
        };
        let plan = preview_with_fx_quotes(&report, request, now(), &books, &buffs, fx.as_ref())
            .map_err(AppError::Invalid)?;
        Ok(plan)
    }
}
