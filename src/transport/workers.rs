use super::http::{ApiError, input, same_origin};
use crate::{
    application::{
        ledger::LedgerStore,
        notify::NotifyService,
        ports::{CredentialStore, GuardSecretStore, SessionVerifier},
        workers::BackgroundWorkers,
    },
    domain::{
        notify::NotifyConfig,
        workers::SchedulerSnapshot,
    },
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State, rejection::JsonRejection},
    http::StatusCode,
    routing::{get, post},
};
use serde_json::{Value, json};
use std::sync::Arc;

struct WorkersTransportState<R, C, SV, GV> {
    workers: Arc<BackgroundWorkers<R, C, SV, GV>>,
    notify: Arc<NotifyService>,
}

impl<R, C, SV, GV> Clone for WorkersTransportState<R, C, SV, GV> {
    fn clone(&self) -> Self {
        Self {
            workers: self.workers.clone(),
            notify: self.notify.clone(),
        }
    }
}

pub fn router<R, C, SV, GV>(
    workers: Arc<BackgroundWorkers<R, C, SV, GV>>,
    notify: Arc<NotifyService>,
) -> Router
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    let state = WorkersTransportState { workers, notify };
    Router::new()
        .route("/api/workers", get(workers_status::<R, C, SV, GV>))
        .route("/api/workers/{name}/trigger", post(trigger_worker::<R, C, SV, GV>))
        .route(
            "/api/settings/notify",
            get(get_notify_settings::<R, C, SV, GV>).put(update_notify_settings::<R, C, SV, GV>),
        )
        .route("/api/settings/notify/test", post(send_test_notify::<R, C, SV, GV>))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(axum::middleware::from_fn(same_origin))
        .layer(axum::middleware::from_fn(
            |r: axum::extract::Request, n: axum::middleware::Next| async move {
                let mut response = n.run(r).await;
                response
                    .headers_mut()
                    .insert("cache-control", "no-store".parse().unwrap());
                response
            },
        ))
        .with_state(state)
}

async fn workers_status<R, C, SV, GV>(
    State(st): State<WorkersTransportState<R, C, SV, GV>>,
) -> Result<Json<SchedulerSnapshot>, ApiError>
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    Ok(Json(st.workers.snapshot().await))
}

async fn trigger_worker<R, C, SV, GV>(
    State(st): State<WorkersTransportState<R, C, SV, GV>>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError>
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    st.workers
        .trigger(&name)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(json!({
        "success": true,
        "message": format!("已手动下发任务调度请求: {name}"),
    })))
}

async fn get_notify_settings<R, C, SV, GV>(
    State(st): State<WorkersTransportState<R, C, SV, GV>>,
) -> Result<Json<NotifyConfig>, ApiError>
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    Ok(Json(st.notify.get_masked_config().await))
}

async fn update_notify_settings<R, C, SV, GV>(
    State(st): State<WorkersTransportState<R, C, SV, GV>>,
    body: Result<Json<NotifyConfig>, JsonRejection>,
) -> Result<Json<NotifyConfig>, ApiError>
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    let new_cfg = input(body)?;
    let saved = st
        .notify
        .update_config(new_cfg)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))))?;
    let mut masked = saved;
    if !masked.pushplus_token.is_empty() {
        let t = &masked.pushplus_token;
        masked.pushplus_token = if t.len() > 8 {
            format!("{}****{}", &t[..4], &t[t.len() - 4..])
        } else {
            "****".into()
        };
    }
    Ok(Json(masked))
}

async fn send_test_notify<R, C, SV, GV>(
    State(st): State<WorkersTransportState<R, C, SV, GV>>,
) -> Result<Json<Value>, ApiError>
where
    R: LedgerStore + 'static,
    C: CredentialStore + 'static,
    SV: SessionVerifier + 'static,
    GV: GuardSecretStore + 'static,
{
    st.notify
        .send_test()
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, Json(json!({"error": e.to_string()}))))?;
    Ok(Json(json!({
        "success": true,
        "message": "✓ 微信测试通知发送成功，请在手机微信中查收！",
    })))
}
