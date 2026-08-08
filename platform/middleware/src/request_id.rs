//! 请求追踪 ID：生成（若客户端未携带）+ 回写到响应头。

use http::HeaderName;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// 请求 ID 头名称。
/// 在此定义唯一出处，避免多处各写一份字符串字面量导致漂移。
pub const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn layers() -> (SetRequestIdLayer<MakeRequestUuid>, PropagateRequestIdLayer) {
    let header_name = HeaderName::from_static(REQUEST_ID_HEADER);
    (
        SetRequestIdLayer::new(header_name.clone(), MakeRequestUuid),
        PropagateRequestIdLayer::new(header_name),
    )
}
