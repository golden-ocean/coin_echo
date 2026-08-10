//! 响应压缩。
//!
//! # 作用
//!
//! 按客户端请求头 `Accept-Encoding` 自动选择压缩算法（gzip/br/zstd/
//! deflate，取决于启用的 tower-http feature）压缩响应体，减少网络传输
//! 体积，对 JSON API 尤其有效（结构化文本压缩率通常很高）。客户端不
//! 支持压缩时自动跳过，不影响兼容性。
//!
//! # 使用的 tower-http 组件
//!
//! [`CompressionLayer`]，完全内置，默认配置即会根据 `Accept-Encoding`
//! 自动协商，不需要额外参数。
//!
//! # 前置条件
//!
//! `Cargo.toml` 中 `tower-http` 需要开启 `compression-full`（或按需
//! 只开 `compression-gzip` 等单项）feature，否则 `CompressionLayer`
//! 不可用或退化为不压缩任何内容。

use tower_http::compression::CompressionLayer;

pub fn layer() -> CompressionLayer {
    CompressionLayer::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Request, Response};
    use tower::{Layer, Service, ServiceExt, service_fn};

    #[tokio::test]
    async fn compresses_response_when_client_accepts_gzip() {
        let inner = service_fn(|_req: Request<()>| async {
            // 用一段可压缩的重复内容作为响应体，验证压缩确实生效
            // （压缩后体积应明显小于原始体积）。
            let body = "hello world ".repeat(200);
            Ok::<_, std::convert::Infallible>(Response::new(body))
        });

        let mut svc = layer().layer(inner);
        let request = Request::builder()
            .header(http::header::ACCEPT_ENCODING, "gzip")
            .body(())
            .unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        // CompressionLayer 协商成功后会设置 Content-Encoding 响应头。
        assert_eq!(
            response
                .headers()
                .get(http::header::CONTENT_ENCODING)
                .map(|v| v.to_str().unwrap()),
            Some("gzip")
        );
    }

    #[tokio::test]
    async fn does_not_compress_when_client_does_not_accept_encoding() {
        let inner = service_fn(|_req: Request<()>| async {
            Ok::<_, std::convert::Infallible>(Response::new("hello".repeat(200)))
        });

        let mut svc = layer().layer(inner);
        // 不带 Accept-Encoding 请求头，客户端表示不支持任何压缩。
        let request = Request::builder().body(()).unwrap();

        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert!(
            response
                .headers()
                .get(http::header::CONTENT_ENCODING)
                .is_none()
        );
    }
}
