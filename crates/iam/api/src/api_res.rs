//! `platform_kernel::http::Res<T>` 的 axum `IntoResponse` 适配。

use axum::Json;
use axum::response::{IntoResponse, Response};
use platform_kernel::http::Res;
use platform_middleware::RequestContext;
use serde::Serialize;

/// 包装类型：让 `Res<T>` 能直接作为 handler 返回值。
pub struct ApiOk<T: Serialize>(pub Res<T>);

impl<T: Serialize> ApiOk<T> {
    /// 构造带数据的成功响应，trace_id 自动从当前请求上下文读取。
    pub fn data(data: T) -> Self {
        Self(Res::ok(data, current_trace_id()))
    }
}

impl ApiOk<()> {
    /// 构造无数据的成功响应（DELETE、状态变更类接口）。
    pub fn empty() -> Self {
        Self(Res::empty(current_trace_id()))
    }
}

impl<T: Serialize> IntoResponse for ApiOk<T> {
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}

/// 从当前请求上下文读取非空 trace_id；无上下文或为空时返回 `None`。
fn current_trace_id() -> Option<String> {
    let ctx = RequestContext::current_or_default();
    (!ctx.trace_id.is_empty()).then_some(ctx.trace_id)
}
