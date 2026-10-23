use super::*;
use crate::{
    application::{
        inventory::collect,
        ports::{InventoryProvider, InventoryReadError as E, InventoryRepository},
    },
    domain::inventory::*,
};

impl<R: AccountRepository + InventoryRepository, C: CredentialStore, V: SessionVerifier>
    Accounts<R, C, V>
{
    pub(crate) async fn read_inventory<P: InventoryProvider>(
        &self,
        id: &str,
        request: InventoryRequest,
        provider: &P,
    ) -> Result<InventorySnapshot, AppError> {
        let _guard = self.gate.lock().await;
        let account = self.account(id).await?;
        let mut session = self
            .repo
            .session(id.into(), Platform::Steam)
            .await?
            .ok_or(AppError::Missing("请先导入并验证 Steam 登录态"))?;
        let at = now();
        if session.view.status != SessionStatus::Valid
            || session.view.subject.as_deref() != Some(&account.steam_id)
            || session
                .view
                .checked_at
                .is_none_or(|t| t > at || at - t > 300)
        {
            return Err(AppError::Conflict(
                "需要最近五分钟内身份匹配的 Steam 在线核验",
            ));
        }
        let retry_at = self
            .repo
            .inventory_retry()
            .await?
            .max(session.view.retry_at);
        if retry_at > at {
            return Err(AppError::Inventory(E::RateLimited(retry_at)));
        }
        let Some(secret) = self.vault.get(session.credential_key.clone()).await? else {
            session.view.status = SessionStatus::Missing;
            session.view.message = "凭据存储缺失，请重新导入".into();
            self.repo.save_session(id.into(), session).await?;
            return Err(AppError::Conflict("凭据存储缺失，请重新导入"));
        };
        secret
            .validate(Platform::Steam, &account.steam_id)
            .map_err(|_| AppError::Conflict("Steam 凭据身份与账户不符，请重新导入"))?;
        let started_at = now();
        let result = collect(provider, &self.repo, &account.steam_id, &secret, &request).await;
        match result {
            Ok((assets, coverage)) => Ok(InventorySnapshot {
                id: uuid::Uuid::new_v4().to_string(),
                account_id: id.into(),
                steam_id: account.steam_id,
                credential_generation: session.view.generation,
                request,
                started_at,
                observed_at: now(),
                source: "steam_authenticated_inventory".into(),
                platform_verified: false,
                coverage,
                assets,
                warnings: vec![
                    "线上库存协议尚未实测；资产属于请求账户的路径绑定，非响应签名证明".into(),
                    "双区分页为分时观察，不是原子库存；预算截断时不能作为完整持仓".into(),
                    "不含市场挂牌、未决收货与未决采购；不替换手工仓位，不证明零风险敞口".into(),
                    "交易/市场标志仅为平台报告；解锁时间未知，不靠本地倒计时放行".into(),
                ],
            }),
            Err(AppError::Inventory(error)) => {
                match error {
                    E::RateLimited(secs) => {
                        let retry = now() + secs.clamp(300, 604800);
                        self.repo.set_inventory_retry(retry).await?;
                        session.view.retry_at = retry;
                        session.view.message = error.message().into();
                        self.repo.save_session(id.into(), session).await?;
                        return Err(AppError::Inventory(E::RateLimited(retry)));
                    }
                    E::Expired => session.view.status = SessionStatus::Invalid,
                    E::CredentialsChanged => {
                        session.view.status = SessionStatus::Unverified;
                        session.view.generation = uuid::Uuid::new_v4().to_string();
                    }
                    // 403 may mean a private inventory, not an expired identity.
                    _ => return Err(AppError::Inventory(error)),
                }
                session.view.message = error.message().into();
                self.repo.save_session(id.into(), session).await?;
                Err(AppError::Inventory(error))
            }
            Err(other) => Err(other),
        }
    }
}
