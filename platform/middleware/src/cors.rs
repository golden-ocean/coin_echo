//! CORS：跨域资源共享策略。使用 tower-http 内置的 [`CorsLayer`]，
//! 允许的来源从配置读取，不写死。

use http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct CorsConfig {
    /// 允许的来源列表，逗号分隔。为空表示不设置
    /// `Access-Control-Allow-Origin`（即拒绝跨域）。生产环境务必显式配置，
    /// 不要用 `*`——那样无法配合携带凭证（cookie）的跨域请求。
    #[serde(default)]
    pub allowed_origins: String,
}

pub fn layer(settings: &CorsConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = settings
        .allowed_origins
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    let layer = if origins.is_empty() {
        tracing::warn!("未配置 MIDDLEWARE_CORS_ALLOWED_ORIGINS，跨域请求将被浏览器拒绝");
        CorsLayer::new()
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                // Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers(tower_http::cors::Any)
            .allow_credentials(true)
    };

    layer
}
