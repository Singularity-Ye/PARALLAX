//! A lease on the existing Accounts gate, not a separate authentication state machine.
use super::*;
use crate::application::ports::{SteamReadError as E, SteamReadPolicy, SteamReadScope};

pub(crate) struct SteamAccess<'a, R> {
    _gate: tokio::sync::MutexGuard<'a, ()>,
    repo: &'a R,
    scope: SteamReadScope,
    pub account: Account,
    pub session: StoredSession,
    pub secret: Credentials,
}
impl<R: AccountRepository + SteamReadPolicy, C: CredentialStore, V: SessionVerifier>
    Accounts<R, C, V>
{
    pub(crate) async fn steam_access(
        &self,
        id: &str,
        scope: SteamReadScope,
    ) -> Result<SteamAccess<'_, R>, AppError> {
        let gate = self.gate.lock().await;
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
        let retry = self
            .repo
            .steam_read_retry(scope)
            .await?
            .max(session.view.retry_at);
        if retry > at {
            return Err(AppError::SteamRead(E::RateLimited(retry)));
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
        Ok(SteamAccess {
            _gate: gate,
            repo: &self.repo,
            scope,
            account,
            session,
            secret,
        })
    }
}
impl<R: AccountRepository + SteamReadPolicy> SteamAccess<'_, R> {
    pub async fn finish<T>(&mut self, result: Result<T, AppError>) -> Result<T, AppError> {
        let Err(AppError::SteamRead(error)) = result else {
            return result;
        };
        match error {
            E::RateLimited(secs) => {
                let retry = now() + secs.clamp(300, 604800);
                self.repo.set_steam_read_retry(self.scope, retry).await?;
                self.session.view.retry_at = retry;
                self.session.view.message = error.message().into();
                self.repo
                    .save_session(self.account.id.clone(), self.session.clone())
                    .await?;
                return Err(AppError::SteamRead(E::RateLimited(retry)));
            }
            E::Expired => self.session.view.status = SessionStatus::Invalid,
            E::CredentialsChanged => {
                self.session.view.status = SessionStatus::Unverified;
                self.session.view.generation = uuid::Uuid::new_v4().to_string();
            }
            _ => return Err(AppError::SteamRead(error)),
        }
        self.session.view.message = error.message().into();
        self.repo
            .save_session(self.account.id.clone(), self.session.clone())
            .await?;
        Err(AppError::SteamRead(error))
    }
}
