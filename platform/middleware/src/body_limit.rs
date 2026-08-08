//! 请求体大小限制：防止未认证的超大请求体耗尽内存。
//! 使用 tower-http 内置的 [`RequestBodyLimitLayer`]。

use tower_http::limit::RequestBodyLimitLayer;

const DEFAULT_MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

/// 中间件工厂。
///
/// `limit_bytes` 为零或未传参时使用默认值 `2 MiB`。
pub fn layer(max_bytes: usize) -> RequestBodyLimitLayer {
    let limit = if max_bytes == 0 {
        DEFAULT_MAX_BODY_SIZE
    } else {
        max_bytes
    };
    RequestBodyLimitLayer::new(limit)
}
