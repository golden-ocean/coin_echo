//! 请求追踪 ID：生成（若客户端未携带）+ 回写到响应头。
//!
//! # 使用的 tower-http 组件
//!
//! - [`SetRequestIdLayer`]：请求进入时，若请求头里已带 `x-request-id`
//!   则保留原值（便于上游网关/客户端自己指定追踪 ID 贯穿多级调用），
//!   否则用 [`MakeRequestUuid`] 生成一个新的 UUID 并写入请求扩展。
//! - [`PropagateRequestIdLayer`]：把请求扩展里的 ID 复制到响应头，让
//!   客户端能在响应里看到本次调用对应的追踪 ID，用于排障时提供给
//!   技术支持。
//!
//! 两层必须按此顺序应用（`set` 在前、`propagate` 在后）——这是
//! tower-http 官方文档给出的标准搭配，`propagate` 依赖 `set` 已经把 ID
//! 写入了请求扩展。
//!
//! # 为什么单独抽出这个头名常量
//!
//! [`REQUEST_ID_HEADER`] 会被多处引用：本文件、`trace.rs`（访问日志读取
//! 请求 ID 放进 span）、 `RequestContext`（把请求 ID
//! 当作 trace_id 传给 `ProblemDetails`）。定义在这里作为唯一出处，避免
//! 多处各写一份 `"x-request-id"` 字符串字面量导致后续修改遗漏。

use http::HeaderName;
use tower::layer::util::Stack;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};

/// 请求 ID 头名称。
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// 返回组合好的 request-id 中间件层（`Stack` 把两层合并成一个可直接
/// `.layer()` 的整体，调用方不需要关心内部有几层、顺序如何）。
pub fn layer() -> Stack<PropagateRequestIdLayer, SetRequestIdLayer<MakeRequestUuid>> {
    let header_name = HeaderName::from_static(REQUEST_ID_HEADER);
    Stack::new(
        PropagateRequestIdLayer::new(header_name.clone()), // inner（后执行）
        SetRequestIdLayer::new(header_name, MakeRequestUuid), // outer（先执行）
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Request, Response};
    use tower::{Layer, Service, ServiceExt, service_fn};

    fn echo_request_id_service() -> impl tower::Service<
        Request<()>,
        Response = Response<()>,
        Error = std::convert::Infallible,
        Future = impl std::future::Future<Output = Result<Response<()>, std::convert::Infallible>>
                 + Send,
    > + Clone {
        service_fn(|_req: Request<()>| async {
            Ok::<_, std::convert::Infallible>(Response::new(()))
        })
    }

    #[tokio::test]
    async fn generates_request_id_when_client_did_not_provide_one() {
        let mut svc = layer().layer(echo_request_id_service());
        let request = Request::builder().uri("/x").body(()).unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        let id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("响应头应包含生成的请求 ID");
        assert!(!id.to_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn preserves_client_provided_request_id() {
        let mut svc = layer().layer(echo_request_id_service());
        let request = Request::builder()
            .uri("/x")
            .header(REQUEST_ID_HEADER, "client-supplied-id-123")
            .body(())
            .unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER).unwrap(),
            "client-supplied-id-123"
        );
    }
}
