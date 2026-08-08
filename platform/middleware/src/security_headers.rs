//! 安全响应头：`X-Content-Type-Options`、`X-Frame-Options`、
//! `Referrer-Policy` 等 OWASP 建议的基线响应头。
//!
//! tower-http 没有一个现成的“SecurityHeadersLayer”，但提供了通用的
//! [`SetResponseHeaderLayer`] 原语；按需堆叠几次即可覆盖这个需求，
use http::{HeaderName, HeaderValue};
use tower_http::set_header::SetResponseHeaderLayer;

pub fn layers() -> [SetResponseHeaderLayer<HeaderValue>; 4] {
    [
        SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ),
        SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ),
        SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("no-referrer"),
        ),
        SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        ),
    ]
}
