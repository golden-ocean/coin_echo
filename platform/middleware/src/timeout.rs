//! 请求超时：超过指定时长未完成的请求由服务端主动中断，避免慢请求
//! 无限占用连接。使用 tower-http 内置的 [`TimeoutLayer`]。

use http::StatusCode;
use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

pub fn layer(timeout_secs: u64) -> TimeoutLayer {
    TimeoutLayer::with_status_code(
        StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(timeout_secs),
    )
}
