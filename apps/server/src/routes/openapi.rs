//! OpenAPI 文档：聚合各路由的 `#[utoipa::path]` 标注，暴露 Swagger UI。

use axum::Router;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use super::health;

#[derive(OpenApi)]
#[openapi(
    paths(health::healthz),
    components(schemas(health::HealthStatus)),
    tags((name = "system", description = "系统级端点（健康检查等）")),
    info(title = "API", version = "0.1.0", description = "服务端 API 文档")
)]
struct ApiDoc;

/// 提供 `/openapi.json` 与 Swagger UI（`/docs`）。
pub fn router() -> Router {
    Router::new().merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
}
