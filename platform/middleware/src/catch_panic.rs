//! panic 兜底：捕获 handler 内部 panic，转换成符合项目错误契约的
//! `application/problem+json` 响应，而不是让连接直接断开或返回
//! tower-http 默认的纯文本 500。
//!
//! # 使用的 tower-http 组件
//!
//! [`CatchPanicLayer`]，内置捕获 panic 的机制（基于
//! `std::panic::catch_unwind`），但默认的响应格式是纯文本，不符合项目
//! 的 `ProblemDetails` 契约，因此用 `CatchPanicLayer::custom` 传入自定义
//! 的响应构造函数——这是"内置骨架 + 自定义回调"的典型用法，不是从零
//! 手写捕获逻辑。
//!
//! # 为什么这里必须依赖 `axum::body::Body`
//!
//! `CatchPanicLayer` 最终要挂载到 axum 的路由链上，axum 内部路由
//! （`axum::routing::Route`）的响应体固定类型是 `axum::body::Body`，
//! 自定义的 panic 处理函数的返回类型必须与之一致。这是本项目中间件
//! 文件里少数几个必须依赖 axum 的例外（另一个是 `rate_limit.rs`），
//! 原因不是设计疏忽，而是这一层组件的本质就是 axum 专属的桥接点。
//!
//! # 与 `platform-telemetry::record_error` 的关系
//!
//! panic 的具体消息（`detail`）属于"只给运维看、不该对外暴露"的内部
//! 实现细节，这里单独用 `tracing::error!` 记录完整消息，不通过
//! `record_error` 统一路径——`record_error` 对 `detail` 的脱敏规则是
//! 照抄 `ProblemDetails` 对外响应体的口径（`ErrorKind::Internal` 类
//! 错误的 `detail` 会被丢弃），而这里恰恰需要日志系统看到完整 panic
//! 消息用于排障，两者诉求相反，因此不适合复用同一条路径。

use std::any::Any;

use axum::body::Body;
use http::{HeaderValue, Response, StatusCode, header};
use platform_kernel::error::{ErrorKind, ErrorMeta};
use platform_kernel::http::ProblemDetails;
use tower_http::catch_panic::CatchPanicLayer;

use crate::context::RequestContext;

type PanicHandler = fn(Box<dyn Any + Send>) -> Response<Body>;

pub fn layer() -> CatchPanicLayer<PanicHandler> {
    CatchPanicLayer::custom(handle_panic as PanicHandler)
}

fn handle_panic(err: Box<dyn Any + Send>) -> Response<Body> {
    let detail = err
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| err.downcast_ref::<String>().cloned());

    // 完整 panic 消息只进日志，不进对外响应体，见模块文档说明。
    tracing::error!(panic.detail = ?detail, "请求处理过程中发生 panic");

    let ctx = RequestContext::current_or_default();
    let problem = ProblemDetails::from_error(&PanicError, "app", ctx.instance, ctx.trace_id);
    let payload = serde_json::to_vec(&problem).unwrap_or_default();

    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )
        .body(Body::from(payload))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

/// panic 场景没有真实的业务错误对象，构造一个固定实现只是为了复用
/// `ProblemDetails::from_error` 的统一渲染逻辑。
struct PanicError;

impl ErrorMeta for PanicError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Internal
    }

    fn code(&self) -> &'static str {
        "internal.panic"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::routing::get;
    use http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn panicking_handler_returns_500_with_problem_json() {
        let app: Router = Router::new()
            .route(
                "/boom",
                get(|| async {
                    panic!("测试用 panic 消息");
                    #[allow(unreachable_code)]
                    ""
                }),
            )
            .layer(layer());

        let response = app
            .oneshot(Request::builder().uri("/boom").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "internal.panic");
    }

    #[tokio::test]
    async fn non_panicking_handler_passes_through_normally() {
        let app: Router = Router::new()
            .route("/ok", get(|| async { "fine" }))
            .layer(layer());

        let response = app
            .oneshot(Request::builder().uri("/ok").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
