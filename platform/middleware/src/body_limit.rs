//! 请求体大小限制。
//!
//! # 作用
//!
//! 拒绝超过指定字节数的请求体，防止未认证的客户端通过发送超大请求体
//! （如几百 MB 的 POST body）耗尽服务器内存——这是资源耗尽类攻击
//! （DoS）的一个常见入口，应该在鉴权、业务逻辑之前的最外层就拦截。
//!
//! # 使用的 tower-http 组件
//!
//! [`RequestBodyLimitLayer`]，完全内置。超过限制时自动返回
//! `413 Payload Too Large`，不需要自定义处理逻辑。
//!
//! # 限制大小的选择
//!
//! 由 [`platform_middleware::MiddlewareConfig::body_limit_bytes`] 统一
//! 配置（前缀 `MIDDLEWARE_`，默认 2 MiB）。如果未来某个业务接口需要
//! 上传更大的文件（如头像、附件），应该在该接口自己的路由上单独覆盖
//! 一个更大的限制，而不是把全局默认值调高——全局默认值应该按"绝大多数
//! 接口的正常请求体大小"来定，个别例外单独处理。

use tower_http::limit::RequestBodyLimitLayer;

const DEFAULT_MAX_BODY_SIZE: usize = 2 * 1024 * 1024; // 2 MiB

pub fn layer(max_bytes: usize) -> RequestBodyLimitLayer {
    let limit = if max_bytes == 0 {
        DEFAULT_MAX_BODY_SIZE
    } else {
        max_bytes
    };
    RequestBodyLimitLayer::new(limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::routing::post;
    use http::{Request, StatusCode};
    use tower::ServiceExt;

    // 用 axum::Router 而非裸 tower::Service 测试：RequestBodyLimitLayer
    // 依赖请求体被完整读取消费的行为，用一个真实的 axum handler
    // （读取 body 到 bytes）更贴近实际使用场景，也更容易验证 413 的
    // 触发条件。

    fn app_with_limit(max_bytes: usize) -> Router {
        Router::new()
            .route(
                "/upload",
                post(|body: axum::body::Bytes| async move { body.len().to_string() }),
            )
            .layer(layer(max_bytes))
    }

    #[tokio::test]
    async fn request_within_limit_is_accepted() {
        let app = app_with_limit(1024);
        let request = Request::builder()
            .method("POST")
            .uri("/upload")
            .body(Body::from(vec![0u8; 100]))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn request_exceeding_limit_is_rejected_with_413() {
        let app = app_with_limit(100);
        let request = Request::builder()
            .method("POST")
            .uri("/upload")
            .body(Body::from(vec![0u8; 1024]))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
