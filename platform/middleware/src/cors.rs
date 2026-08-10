//! CORS（跨域资源共享）。
//!
//! # 作用
//!
//! 浏览器默认禁止网页脚本向不同源（协议+域名+端口）发起的请求读取响应，
//! 这个中间件通过设置 `Access-Control-Allow-*` 系列响应头，明确告知
//! 浏览器"哪些来源、方法、头部允许跨域访问本服务"。前后端分离架构下
//! （前端独立域名部署），这是必需的，否则前端 fetch/XHR 请求会被浏览器
//! 直接拦截。
//!
//! # 使用的 tower-http 组件
//!
//! [`CorsLayer`]，完全内置，本文件只是按配置组装允许的来源列表。
//!
//! # 为什么不用 `Any`（允许所有来源）
//!
//! `CorsLayer::new().allow_origin(Any)` 虽然最省事，但有一个关键限制：
//! **`Access-Control-Allow-Origin: *` 与 `Access-Control-Allow-Headers: *`
//! 均无法与 `Access-Control-Allow-Credentials: true` 同时使用**——这是浏览器
//! CORS 规范本身的限制。如果前端需要携带 Cookie/Authorization 头做跨域请求
//! （`allow_credentials(true)`），就必须显式指定允许的 Origin 和 Headers。
//!
//! # 未配置来源时的降级行为
//!
//! [`CorsConfig::allowed_origins`] 为空时，不设置任何
//! `Access-Control-Allow-Origin`——效果等同于拒绝所有跨域请求，同时打
//! 一条 warn 日志提醒运维这是配置缺失而非有意为之。

use http::{HeaderName, HeaderValue, Method, header};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

/// CORS 相关配置，作为 [`super::config::MiddlewareConfig`] 的嵌套字段。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorsConfig {
    /// 允许的来源列表，逗号分隔（如
    /// `https://app.example.com,https://admin.example.com`）。
    #[serde(default)]
    pub allowed_origins: String,
}

pub fn layer(cfg: &CorsConfig) -> CorsLayer {
    let origins: Vec<HeaderValue> = cfg
        .allowed_origins
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect();

    if origins.is_empty() {
        tracing::warn!("MIDDLEWARE_CORS_ALLOWED_ORIGINS 未配置，跨域请求将被浏览器拒绝");
        CorsLayer::new()
    } else {
        CorsLayer::new()
            .allow_origin(origins)
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ])
            // 1. 如果希望放行标准通用头，可配置具体 HeaderName：
            .allow_headers([
                header::AUTHORIZATION,
                header::CONTENT_TYPE,
                header::ACCEPT,
                header::ORIGIN,
                HeaderName::from_static("x-requested-with"),
            ])
            // 2. 如果前端经常传递自定义请求头（如 X-Trace-Id 等），可取消上面一行，使用镜像请求头：
            // .allow_headers(tower_http::cors::AllowHeaders::mirror_request())
            .allow_credentials(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_produces_layer_without_panicking() {
        let settings = CorsConfig::default();
        let _layer = layer(&settings);
    }

    #[test]
    fn valid_config_builds_layer_without_panicking() {
        let settings = CorsConfig {
            allowed_origins: "http://localhost:3000, https://app.example.com".to_string(),
        };
        // 验证带有 Credentials 时的合法配置组合不再抛出 Panic
        let _layer = layer(&settings);
    }

    #[test]
    fn malformed_origin_entries_are_silently_skipped() {
        let settings = CorsConfig {
            allowed_origins: "https://valid.example.com, not a valid header value\n".to_string(),
        };
        let _layer = layer(&settings);
    }
}
