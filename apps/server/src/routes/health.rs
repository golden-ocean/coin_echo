//! 健康检查路由。真实探测数据库/缓存连通性，不是只返回 200。

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use utoipa::ToSchema;

use crate::AppState;

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthStatus {
    pub status: &'static str,
    pub database: bool,
    pub cache: bool,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/healthz", get(healthz))
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "全部依赖正常", body = HealthStatus),
        (status = 503, description = "至少一项依赖不可用", body = HealthStatus),
    ),
    tag = "system"
)]
pub(crate) async fn healthz(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<HealthStatus>) {
    let database = state.pools.health_check().await.is_ok();
    let cache = state.cache.health_check().await.is_ok();
    let all_ok = database && cache;

    let body = HealthStatus {
        status: if all_ok { "ok" } else { "degraded" },
        database,
        cache,
    };
    let code = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (code, Json(body))
}
