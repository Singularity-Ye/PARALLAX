use super::ports::{CatalogError, CatalogProvider};
use crate::domain::catalog::*;
use std::sync::Arc;
use tokio::sync::Mutex;
pub struct Catalog<P> {
    provider: P,
    next_read: Mutex<u64>,
}
impl<P: CatalogProvider> Catalog<P> {
    pub fn new(provider: P) -> Arc<Self> {
        Arc::new(Self {
            provider,
            next_read: Mutex::new(0),
        })
    }
    pub async fn search(&self, request: CatalogRequest) -> Result<CatalogPage, CatalogError> {
        request.validate().map_err(CatalogError::Invalid)?;
        let mut gate = self.next_read.try_lock().map_err(|_| CatalogError::Busy)?;
        let now = crate::domain::selection::now();
        if now < *gate {
            return Err(CatalogError::RateLimited(*gate));
        }
        *gate = now + 15;
        let result = self.provider.search(&request).await;
        if let Err(CatalogError::RateLimited(until)) = &result {
            *gate = (*gate).max(*until);
        }
        result
    }
}
