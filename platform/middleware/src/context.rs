//! 请求级上下文的跨层传递：当前请求的路径（用于 `ProblemDetails::instance`）
//! 与 trace_id。
//!
//! # 为什么放在 `platform-middleware` 而不是各中间件自己处理
//!
//! `catch_panic`/`jwt`/`casbin`/`rate_limit` 等多个中间件都需要在构造
//! `ProblemDetails::from_error(...)` 时填入 `instance`/`trace_id`，这是
//! 一份被多处共享的能力——不是"只服务一个消费者"的东西，应该放在
//! crate 内部的共享位置，而不是各中间件文件各自维护一份。
//!
//! # 为什么用 `tokio::task_local!` 而不是显式传参
//!
//! 各中间件构造错误响应的代码路径（如 `catch_panic::handle_panic`）
//! 没有办法拿到原始 `http::Request`——它们的函数签名由
//! `CatchPanicLayer::custom`/`tower::Service::call` 等外部 trait 决定，
//! 不受我们控制，无法额外插入一个"当前请求上下文"参数。
//!
//! `tokio::task_local!` 利用了一个事实：一次请求从最外层中间件到内层
//! handler 的整条调用链，正常情况下运行在**同一个 tokio task** 里
//! （除非某处显式 `tokio::spawn`）。[`RequestContextLayer`] 在请求进入
//! 时把上下文存进 task-local，`.await` 整个后续调用链；调用链上任意
//! 位置都能通过 [`RequestContext::current`] 取回，不需要显式传参。这是
//! tower/tokio 生态里传递请求作用域数据的标准手法。
//!
//! # 应用位置
//!
//! 必须在 [`super::request_id`] 之后（`Router::layer()` 调用顺序上更早
//! 执行，即更外层）——依赖请求头里已经有 `x-request-id`，这个由
//! request_id 中间件生成/透传。同时要在所有会用到
//! [`RequestContext::current`] 的中间件（catch_panic/jwt/casbin/
//! rate_limit）之外，才能保证它们执行时上下文已经安装好。

use std::future::Future;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll};

use http::Request;
use tower::{Layer, Service};

use super::request_id::REQUEST_ID_HEADER;

tokio::task_local! {
    static REQUEST_CONTEXT: RequestContext;
}

/// 请求级上下文快照。
#[derive(Debug, Clone, Default)]
pub struct RequestContext {
    /// 触发本次调用的请求路径（原始 URI path，非路由模式）。
    pub instance: String,
    /// 链路追踪 ID，取自 `x-request-id` 请求头。
    pub trace_id: String,
}

impl RequestContext {
    /// 取回当前 task 的请求上下文。中间件未安装时（如单元测试里裸调用
    /// 内层 service，或者 [`RequestContextLayer`] 因配置被关闭）返回
    /// `None`，调用方应落回 `RequestContext::default()`（即
    /// `instance`/`trace_id` 均为空字符串，`ProblemDetails` 仍能正常
    /// 构造，只是这两个字段为空，不影响响应本身的正确性）。
    #[must_use]
    pub fn current() -> Option<Self> {
        REQUEST_CONTEXT.try_with(Clone::clone).ok()
    }

    /// 便捷方法：取回上下文，未安装时返回默认值而非 `None`。多数调用
    /// 点（构造 `ProblemDetails` 时）不需要区分"未安装"和"已安装但为
    /// 空"，直接用这个方法更简洁。
    #[must_use]
    pub fn current_or_default() -> Self {
        Self::current().unwrap_or_default()
    }

    /// 在给定上下文中执行异步代码块。
    ///
    /// 主要用于单元测试中模拟请求上下文。生产代码应通过
    /// [`RequestContextLayer`] 自动注入，不应手动调用此方法。
    pub async fn scope<F, T>(ctx: Self, f: F) -> T
    where
        F: std::future::Future<Output = T>,
    {
        REQUEST_CONTEXT.scope(ctx, f).await
    }
}

/// 安装请求上下文的 tower 中间件。
#[derive(Debug, Clone, Default)]
pub struct RequestContextLayer;

impl<S> Layer<S> for RequestContextLayer {
    type Service = RequestContextService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestContextService { inner }
    }
}

#[derive(Debug, Clone)]
pub struct RequestContextService<S> {
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for RequestContextService<S>
where
    S: Service<Request<ReqBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<S::Response, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut TaskContext<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let ctx = RequestContext {
            instance: request.uri().path().to_string(),
            trace_id: request
                .headers()
                .get(REQUEST_ID_HEADER)
                .and_then(|v| v.to_str().ok())
                .unwrap_or_default()
                .to_string(),
        };

        let mut inner = self.inner.clone();
        Box::pin(REQUEST_CONTEXT.scope(ctx, async move { inner.call(request).await }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Response;
    use tower::{ServiceExt, service_fn};

    #[tokio::test]
    async fn context_is_readable_inside_inner_service() {
        let inner = service_fn(|_req: Request<()>| async move {
            let ctx = RequestContext::current().expect("上下文应已安装");
            Ok::<_, std::convert::Infallible>(Response::new(ctx.instance))
        });

        let mut svc = RequestContextLayer.layer(inner);
        let request = Request::builder()
            .uri("/v1/users/1")
            .header(REQUEST_ID_HEADER, "trace-abc")
            .body(())
            .unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.into_body(), "/v1/users/1");
    }

    #[tokio::test]
    async fn trace_id_extracted_from_request_id_header() {
        let inner = service_fn(|_req: Request<()>| async move {
            let ctx = RequestContext::current().expect("上下文应已安装");
            Ok::<_, std::convert::Infallible>(Response::new(ctx.trace_id))
        });

        let mut svc = RequestContextLayer.layer(inner);
        let request = Request::builder()
            .uri("/x")
            .header(REQUEST_ID_HEADER, "trace-abc")
            .body(())
            .unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.into_body(), "trace-abc");
    }

    #[tokio::test]
    async fn missing_request_id_header_yields_empty_trace_id_not_panic() {
        let inner = service_fn(|_req: Request<()>| async move {
            let ctx = RequestContext::current().expect("上下文应已安装");
            Ok::<_, std::convert::Infallible>(Response::new(ctx.trace_id))
        });

        let mut svc = RequestContextLayer.layer(inner);
        let request = Request::builder().uri("/x").body(()).unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.into_body(), "");
    }

    #[test]
    fn current_returns_none_without_middleware() {
        assert!(RequestContext::current().is_none());
    }

    #[test]
    fn current_or_default_returns_empty_context_without_middleware() {
        let ctx = RequestContext::current_or_default();
        assert_eq!(ctx.instance, "");
        assert_eq!(ctx.trace_id, "");
    }
}
