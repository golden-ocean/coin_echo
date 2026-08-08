//! 访问日志：`ReqBody` 泛型化，本文件不认识 axum；具体 body 类型由调用方
//! （`apply.rs`）通过 turbofish 指定，如 `trace::layer::<axum::body::Body>()`。

use http::Request;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::request_id::REQUEST_ID_HEADER;

type SpanFn<ReqBody> = fn(&Request<ReqBody>) -> tracing::Span;

fn make_span<ReqBody>(request: &Request<ReqBody>) -> tracing::Span {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_string();

    tracing::info_span!(
        "http_request",
        method = %request.method(),
        path = %request.uri().path(),
        request_id = %request_id,
    )
}

pub fn layer<ReqBody>() -> TraceLayer<
    SharedClassifier<ServerErrorsAsFailures>,
    SpanFn<ReqBody>,
    DefaultOnRequest,
    DefaultOnResponse,
> {
    TraceLayer::new_for_http()
        .make_span_with(make_span::<ReqBody> as SpanFn<ReqBody>)
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .include_headers(false),
        )
}
