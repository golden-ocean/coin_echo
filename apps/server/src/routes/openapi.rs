//! OpenAPI 文档：聚合各路由的 `#[utoipa::path]` 标注。
//!
//! 方式：
//! 1. HTTP 端点 `/openapi.json`，纯 axum 实现，前端代码生成工具
//!    （openapi-generator/orval 等）直接拉取。
//! 2. [`write_spec_to_file`]，供启动时或独立命令把 spec 落盘成文件，
//!    可提交进仓库或作为 CI 产物。

use std::path::Path;

use axum::routing::get;
use axum::{Json, Router};
use utoipa::OpenApi;

use super::health;

#[derive(OpenApi)]
#[openapi(
    paths(health::healthz),
    components(schemas(health::HealthStatus)),
    tags((name = "system", description = "系统级端点（健康检查等）")),
    info(title = "API", version = "0.1.0", description = "服务端 API 文档")
)]
struct ApiDoc;

/// 提供 `/openapi.json` 端点。
pub fn router() -> Router {
    Router::new().route("/openapi.json", get(serve_spec))
}

async fn serve_spec() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}

/// 把当前 OpenAPI spec 序列化并写入指定路径。
///
/// 用途：CI 产物导出、提交一份静态 spec 文件供前端离线使用。不在
/// [`crate::bootstrap::run::run`] 的正常启动路径里强制调用——写文件失败
/// 不该阻止服务启动，是否需要在启动时顺带写一份，由调用方决定。
pub fn write_spec_to_file(path: impl AsRef<Path>) -> anyhow::Result<()> {
    let spec = ApiDoc::openapi();
    let json = spec.to_pretty_json()?;
    std::fs::write(path, json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_serializes_to_valid_json() {
        let spec = ApiDoc::openapi();
        let json = spec.to_pretty_json().unwrap();
        assert!(json.contains("\"/healthz\""));
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
    }

    #[test]
    fn write_spec_to_file_creates_readable_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("openapi.json");

        write_spec_to_file(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"/healthz\""));
    }
}
