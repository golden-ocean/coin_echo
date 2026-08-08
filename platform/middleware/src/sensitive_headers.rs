//! 敏感头脱敏：标记 `Authorization`/`Cookie` 等头为"敏感"，使其不出现在
//! 访问日志里。

use http::header;
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};

const SENSITIVE_HEADERS: [http::HeaderName; 3] =
    [header::AUTHORIZATION, header::COOKIE, header::SET_COOKIE];

pub fn request_layer() -> SetSensitiveRequestHeadersLayer {
    SetSensitiveRequestHeadersLayer::new(SENSITIVE_HEADERS)
}

pub fn response_layer() -> SetSensitiveResponseHeadersLayer {
    SetSensitiveResponseHeadersLayer::new(SENSITIVE_HEADERS)
}
