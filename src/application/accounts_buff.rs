//! Shared lease on Accounts: serializes reads with credential replacement/recovery.
use super::*;
use crate::application::ports::BuffReadError as E;
pub(crate) struct BuffAccess<'a, R> {
    _gate: tokio::sync::MutexGuard<'a, ()>,
    repo: &'a R,
    pub account: Account,
    pub session: StoredSession,
    pub secret: Credentials,
}
impl<R: AccountRepository, C: CredentialStore, V: SessionVerifier> Accounts<R, C, V> {
    pub(crate) async fn buff_access(&self, id: &str) -> Result<BuffAccess<'_, R>, AppError> {
        let gate = self.gate.lock().await;
        let account = self.account(id).await?;
        let mut session = self
            .repo
            .session(id.into(), Platform::Buff)
            .await?
            .ok_or(AppError::Missing("请先导入并验证 BUFF 登录态"))?;
        let checked_now = now();
        if session.view.status != SessionStatus::Valid
            || session.view.subject.is_none()
            || session
                .view
                .checked_at
                .is_none_or(|at| at > checked_now || checked_now - at > 300)
        {
            return Err(AppError::Conflict(
                "需要最近五分钟内在线身份验证通过的 BUFF 登录态",
            ));
        }
        let (retry, blocked) = self.repo.buff_policy().await?;
        if blocked {
            return Err(AppError::Buff(E::Challenge));
        }
        if retry > now() || session.view.retry_at > now() {
            return Err(AppError::Buff(E::RateLimited(
                retry.max(session.view.retry_at),
            )));
        }
        let Some(secret) = self.vault.get(session.credential_key.clone()).await? else {
            session.view.status = SessionStatus::Missing;
            session.view.message = "凭据存储缺失，请重新导入".into();
            self.repo.save_session(id.into(), session).await?;
            return Err(AppError::Conflict("凭据存储缺失，请重新导入"));
        };
        // Shares the gate and persistent policy with verification and browser recovery.

        Ok(BuffAccess {
            _gate: gate,
            repo: &self.repo,
            account,
            session,
            secret,
        })
    }
}
impl<R: AccountRepository> BuffAccess<'_, R> {
    pub async fn pace(&self) -> Result<(), AppError> {
        use crate::application::ports::BuffReadPermit;
        match self.repo.try_buff_read(now()).await? {
            BuffReadPermit::Allowed => Ok(()),
            BuffReadPermit::Blocked => Err(AppError::Buff(E::Challenge)),
            BuffReadPermit::CoolingUntil(at) => Err(AppError::Buff(E::RateLimited(at))),
        }
    }
    pub async fn finish<T>(&mut self, result: Result<T, AppError>) -> Result<T, AppError> {
        let Err(AppError::Buff(error)) = result else {
            return result;
        };
        match error {
            E::Challenge => {
                self.repo.set_buff_policy(now() + 30, true).await?;
                self.session.view.status = SessionStatus::VerificationRequired;
            }
            E::RateLimited(secs) => {
                let retry = now() + secs.clamp(300, 604800);
                self.repo.set_buff_policy(retry, false).await?;
                self.session.view.retry_at = retry;
                // This is a read failure, not an identity verification result.
                self.session.view.message = error.message().into();
                self.repo
                    .save_session(self.account.id.clone(), self.session.clone())
                    .await?;
                return Err(AppError::Buff(E::RateLimited(retry)));
            }
            E::Expired => self.session.view.status = SessionStatus::Invalid,
            E::CredentialsChanged => {
                self.session.view.status = SessionStatus::Unverified;
                self.session.view.generation = uuid::Uuid::new_v4().to_string();
            }
            _ => return Err(AppError::Buff(error)),
        }
        self.session.view.message = error.message().into();
        self.repo
            .save_session(self.account.id.clone(), self.session.clone())
            .await?;
        Err(AppError::Buff(error))
    }
}
