//! 请求超时。
//!
//! # 作用
//!
//! 单个请求处理超过指定时长仍未完成时，由服务端主动中断并返回错误，
//! 防止某个慢请求（如下游依赖挂起、死循环）无限占用连接和线程资源，
//! 拖垮整个服务的并发能力。
//!
//! # 使用的 tower-http 组件
//!
//! [`TimeoutLayer`]，完全内置。超时触发时返回 `Ok(Response)`（状态码
//! 由 `with_status_code` 指定，默认对应 `408 Request Timeout`），不会
//! 让 `Service::call` 返回 `Err`——这与很多人直觉里"超时=错误"不同，
//! 是该版本 tower-http 的实现方式：把超时当作一种正常的响应结果对待，
//! 而不是服务级错误，不需要自定义处理逻辑。如果未来需要让超时响应也
//! 符合项目的 `ProblemDetails` 契约（像 `catch_panic`/`rate_limit` 那
//! 样带上结构化 body），需要额外包一层自定义处理，当前先用默认行为，
//! 等有实际需求再升级。
//!
//! # 超时时长的选择
//!
//! 由 [`platform_middleware::MiddlewareConfig::timeout_secs`] 统一配置
//! （前缀 `MIDDLEWARE_`），不在本文件写死——不同部署环境（本地开发 vs
//! 生产）可能需要不同的超时容忍度。

use std::time::Duration;

use http::StatusCode;
use tower_http::timeout::TimeoutLayer;

pub fn layer(timeout_secs: u64) -> TimeoutLayer {
    TimeoutLayer::with_status_code(
        StatusCode::REQUEST_TIMEOUT,
        Duration::from_secs(timeout_secs),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Request, Response};
    use tower::{Layer, Service, ServiceExt, service_fn};

    #[tokio::test]
    async fn fast_request_completes_normally() {
        let inner = service_fn(|_req: Request<()>| async {
            Ok::<_, std::convert::Infallible>(Response::new(()))
        });

        let mut svc = layer(1).layer(inner);
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(()))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn slow_request_is_interrupted_by_timeout() {
        let inner = service_fn(|_req: Request<()>| async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<_, std::convert::Infallible>(Response::new(()))
        });

        let mut svc = layer_from_millis(10).layer(inner);
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(()))
            .await
            .unwrap();

        // 这个版本的 TimeoutLayer 超时后仍返回 Ok(Response)，通过状态码
        // 判断是否触发了超时，而不是断言 Result 是 Err——这是该版本的
        // 既定行为，不是异常情况。
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);
    }

    fn layer_from_millis(millis: u64) -> TimeoutLayer {
        TimeoutLayer::with_status_code(StatusCode::REQUEST_TIMEOUT, Duration::from_millis(millis))
    }
}
