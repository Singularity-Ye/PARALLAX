use super::{
    AppError,
    ports::{FxProvider, FxRepository},
};
use crate::domain::{fx::FxSnapshot, selection::now};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::sync::Mutex;
pub trait FxResolver: Send + Sync {
    fn rates(&self) -> Pin<Box<dyn Future<Output = Result<FxSnapshot, AppError>> + Send + '_>>;
}
pub struct ExchangeRates<P, R> {
    provider: P,
    repo: R,
    gate: Mutex<u64>,
}
impl<P: FxProvider, R: FxRepository> ExchangeRates<P, R> {
    pub fn new(provider: P, repo: R) -> Arc<Self> {
        Arc::new(Self {
            provider,
            repo,
            gate: Mutex::new(0),
        })
    }
    async fn current(&self) -> Result<FxSnapshot, AppError> {
        let mut retry = self.gate.lock().await;
        let at = now();
        if let Some(saved) = self.repo.latest_fx().await?
            && saved.validate(at).is_ok()
        {
            return Ok(saved);
        }
        if *retry > at {
            return Err(AppError::Conflict(
                "汇率刷新失败后暂停五分钟，请稍后重试；未使用过期汇率",
            ));
        }
        *retry = at + 300;
        let snapshot = self.provider.fetch_rates(at).await.map_err(AppError::Fx)?;
        snapshot.validate(now()).map_err(AppError::Fx)?;
        self.repo.insert_fx(snapshot.clone()).await?;
        *retry = 0;
        Ok(snapshot)
    }
}
impl<P: FxProvider, R: FxRepository> FxResolver for ExchangeRates<P, R> {
    fn rates(&self) -> Pin<Box<dyn Future<Output = Result<FxSnapshot, AppError>> + Send + '_>> {
        Box::pin(self.current())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{adapters::sqlite::Store, domain::fx::tests::fixture};
    use std::{
        path::Path,
        sync::atomic::{AtomicUsize, Ordering},
    };
    struct Source {
        calls: Arc<AtomicUsize>,
        fail: bool,
    }
    impl FxProvider for Source {
        async fn fetch_rates(&self, at: u64) -> Result<FxSnapshot, String> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail {
                Err("fixture network error".into())
            } else {
                Ok(fixture(at))
            }
        }
    }
    #[tokio::test]
    async fn concurrent_reads_reuse_immutable_cache_and_reopen_preserves_rates() {
        let path = std::env::temp_dir().join(format!("parallax-fx-{}.db", uuid::Uuid::new_v4()));
        let calls = Arc::new(AtomicUsize::new(0));
        let repo = Store::open(&path).unwrap();
        let fx = ExchangeRates::new(
            Source {
                calls: calls.clone(),
                fail: false,
            },
            repo.clone(),
        );
        let (a, b) = tokio::join!(fx.rates(), fx.rates());
        let a = a.unwrap();
        assert_eq!(a.id, b.unwrap().id);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(repo.insert_fx(a.clone()).await.is_err());
        drop(fx);
        drop(repo);
        let repo = Store::open(&path).unwrap();
        let fx = ExchangeRates::new(
            Source {
                calls: calls.clone(),
                fail: true,
            },
            repo,
        );
        assert_eq!(fx.rates().await.unwrap().id, a.id);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(fx);
        let _ = std::fs::remove_file(path);
    }
    #[tokio::test]
    async fn failed_refresh_does_not_save_fake_rates_or_repeat_requests() {
        let calls = Arc::new(AtomicUsize::new(0));
        let repo = Store::open(Path::new(":memory:")).unwrap();
        let fx = ExchangeRates::new(
            Source {
                calls: calls.clone(),
                fail: true,
            },
            repo.clone(),
        );
        assert!(matches!(fx.rates().await, Err(AppError::Fx(_))));
        assert!(matches!(fx.rates().await, Err(AppError::Conflict(_))));
        assert!(repo.latest_fx().await.unwrap().is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}
