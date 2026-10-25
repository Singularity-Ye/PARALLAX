use super::*;
use crate::{
    application::{
        inventory::collect,
        ports::{InventoryProvider, InventoryRepository, SteamReadScope},
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
        let mut access = self.steam_access(id, SteamReadScope::Inventory).await?;
        let started_at = now();
        let result = collect(
            provider,
            &self.repo,
            &access.account.steam_id,
            &access.secret,
            &request,
        )
        .await;
        let (assets, coverage) = access.finish(result).await?;
        Ok(InventorySnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            account_id: id.into(),
            steam_id: access.account.steam_id.clone(),
            credential_generation: access.session.view.generation.clone(),
            request,
            started_at,
            observed_at: now(),
            source: "steam_authenticated_inventory".into(),
            platform_verified: false,
            coverage,
            assets,
            warnings: vec![
                "资产属于请求账户的路径绑定，非响应签名证明".into(),
                "双区分页为分时观察，不是原子库存；预算截断时不能作为完整持仓".into(),
                "不含市场挂牌、未决收货与未决采购；不替换手工仓位，不证明零风险敞口".into(),
                "交易/市场标志仅为平台报告；解锁时间未知，不靠本地倒计时放行".into(),
            ],
        })
    }
}
