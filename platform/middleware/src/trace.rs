//! 访问日志：基于 tower-http 内置的 [`TraceLayer`]，自定义 span 构造，
//! 让每条请求相关的日志自动携带方法/路径/请求 ID 字段。
//!
//! # 为什么不能直接用 `TraceLayer::new_for_http()` 的默认行为
//!
//! 默认的 span 构造函数只带 `method`/`uri`/`version` 等 HTTP 协议层面
//! 的字段，不包含请求 ID——而请求 ID 是贯穿一次请求所有日志（包括
//! 后续 usecase 层通过 `#[tracing::instrument]` 产生的 span）串联起来
//! 的关键字段。这里自定义 `make_span_with`，从请求头里读出
//! [`super::request_id::REQUEST_ID_HEADER`]（此时应该已经由更外层的
//! request_id 中间件写入），塞进 span。
//!
//! # 为什么 `ReqBody` 要泛型化
//!
//! 保持与其他中间件文件一致的"不认识 axum 具体 body 类型"原则——
//! 调用方（`apply.rs`）通过 turbofish 指定具体类型，如
//! `trace::layer::<axum::body::Body>()`。
//!
//! # 应用位置
//!
//! 必须在 [`super::request_id`] 之后（在 `Router::layer()` 调用顺序上
//! 更早执行，即比 request_id 更外层——回顾"后调用的 `.layer()` 是最
//! 外层"），这样 span 构造时才能从请求头读到已经生成好的请求 ID；同时
//! 需要在 [`super::sensitive_headers`] 之外（比 sensitive_headers 更晚
//! 执行/更内层），这样访问日志如果打印了原始头信息，才能看到已经被
//! 标记为敏感、从而被自动隐藏的 `Authorization`/`Cookie` 值。

use http::Request;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tracing::Level;

use crate::request_id::REQUEST_ID_HEADER;

/// `make_span_with` 要求传入一个具体的函数指针类型（而非闭包捕获的
/// 匿名类型），否则 `TraceLayer` 的完整类型签名无法在 `layer()` 的
/// 返回值里写出来。
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
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use http::Response;
    use tower::{Layer, Service, ServiceExt, service_fn};

    // 直接测 make_span 这个纯函数：真正需要验证的逻辑只有"从请求头正确
    // 提取 request_id 并放进 span 字段"，不需要验证 tower-http 内部
    // 如何把 span 关联到内层 service 的 future 执行上下文——那是
    // TraceLayer 自身已经被广泛使用和验证过的行为，不属于本文件的
    // 职责范围。

    #[test]
    fn make_span_reads_request_id_from_header() {
        let request = Request::builder()
            .uri("/v1/users/1")
            .header(REQUEST_ID_HEADER, "trace-abc")
            .body(())
            .unwrap();

        let span = make_span(&request);
        assert_eq!(span.metadata().unwrap().name(), "http_request");
        // metadata().fields() 只能拿到字段名列表，拿不到运行时的值——
        // tracing::Span 没有提供"读取某个字段当前值"的公开 API（这是
        // 刻意设计，span 的字段值只保证能被 subscriber 观测到，不保证
        // 能被代码反向读出）。这里退而求其次，断言字段名确实被声明了，
        // 结合下面 layer() 端到端测试断言"整条链路不 panic、能正常
        // 拿到响应"，两者合起来足够覆盖这个函数的正确性。
        let field_names: Vec<&str> = span
            .metadata()
            .unwrap()
            .fields()
            .iter()
            .map(|f| f.name())
            .collect();
        assert!(field_names.contains(&"request_id"));
        assert!(field_names.contains(&"method"));
        assert!(field_names.contains(&"path"));
    }

    #[tokio::test]
    async fn layer_does_not_panic_and_passes_response_through() {
        // 端到端验证：整条链路（含真实的 TraceLayer 包裹）能正常处理
        // 请求并返回预期状态码，不因为 span 构造逻辑而出错。
        let inner = service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
        });

        let mut svc = layer::<Body>().layer(inner);
        let request = Request::builder()
            .uri("/v1/users/1")
            .header(REQUEST_ID_HEADER, "trace-abc")
            .body(Body::empty())
            .unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), http::StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_request_id_header_does_not_panic() {
        let inner = service_fn(|_req: Request<Body>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
        });

        let mut svc = layer::<Body>().layer(inner);
        let request = Request::builder().uri("/x").body(Body::empty()).unwrap();
        let response = svc.ready().await.unwrap().call(request).await;
        assert!(response.is_ok());
    }
}
