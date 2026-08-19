use axum::Json;
use axum::response::{IntoResponse, Response};
use platform_kernel::http::Res;
use platform_middleware::RequestContext;
use serde::Serialize;

pub struct ApiOk<T: Serialize>(Res<T>);

impl<T: Serialize> ApiOk<T> {
    pub fn data(data: T) -> Self {
        let trace_id = RequestContext::current_or_default().trace_id.to_string();
        let trace_id = (!trace_id.is_empty()).then_some(trace_id);
        Self(Res::ok(data, trace_id))
    }
}

impl ApiOk<()> {
    pub fn empty() -> Self {
        let trace_id = RequestContext::current_or_default().trace_id.to_string();
        let trace_id = (!trace_id.is_empty()).then_some(trace_id);
        Self(Res::empty(trace_id))
    }
}

impl<T: Serialize> IntoResponse for ApiOk<T> {
    fn into_response(self) -> Response {
        Json(self.0).into_response()
    }
}
