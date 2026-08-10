//! OWASP 基线安全响应头。
//!
//! # 作用
//!
//! 给每个响应附加几个业界公认的安全相关响应头，降低常见 Web 攻击面
//! （点击劫持、MIME 类型混淆嗅探、第三方页面滥用浏览器敏感 API 等）。
//! 这是"零成本"的安全加固——不需要业务代码配合，对所有响应统一生效。
//!
//! | 头 | 值 | 防护目标 |
//! |---|---|---|
//! | `X-Content-Type-Options` | `nosniff` | 阻止浏览器"猜测"响应的 MIME 类型，防止把非脚本文件当脚本执行 |
//! | `X-Frame-Options` | `DENY` | 禁止页面被嵌入 `<iframe>`，防止点击劫持（clickjacking） |
//! | `Referrer-Policy` | `no-referrer` | 跳转到外部站点时不泄露来源 URL（可能包含敏感路径/参数） |
//! | `Permissions-Policy` | 禁用 geolocation/microphone/camera | 明确禁止页面调用这几个敏感浏览器 API，即使被恶意脚本注入也无法启用 |
//!
//! # 使用的 tower-http 组件
//!
//! tower-http 没有一个现成的"SecurityHeadersLayer"打包这几个头，但提供
//! 了通用的 [`SetResponseHeaderLayer`] 原语，按需堆叠几次即可覆盖这个
//! 需求——仍然是"组合内置能力"，不需要为此手写新的 `Service`。
//!
//! # `if_not_present` 而非 `overriding`
//!
//! 用 [`SetResponseHeaderLayer::if_not_present`]：如果某个 handler 出于
//! 特殊需要已经显式设置了这些头（比如某个接口需要允许被 iframe 嵌入），
//! 这里不会覆盖 handler 的显式设置。这是"提供合理默认值，同时不剥夺
//! 业务代码按需覆盖的权利"的标准做法。

use http::{HeaderName, HeaderValue};
use tower::layer::util::Stack;
use tower_http::set_header::SetResponseHeaderLayer;

type SecurityHeadersLayer = Stack<
    SetResponseHeaderLayer<HeaderValue>,
    Stack<
        SetResponseHeaderLayer<HeaderValue>,
        Stack<SetResponseHeaderLayer<HeaderValue>, SetResponseHeaderLayer<HeaderValue>>,
    >,
>;

/// 返回组合好的四个安全响应头层，调用方只需 `.layer(security_headers::layer())`
/// 一次即可全部生效，不需要手动遍历或嵌套调用。
pub fn layer() -> SecurityHeadersLayer {
    let content_type_options = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    let frame_options = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    let referrer_policy = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    let permissions_policy = SetResponseHeaderLayer::if_not_present(
        HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    Stack::new(
        content_type_options,
        Stack::new(
            frame_options,
            Stack::new(referrer_policy, permissions_policy),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{Request, Response};
    use tower::{Layer, Service, ServiceExt, service_fn};

    #[tokio::test]
    async fn response_carries_all_four_security_headers() {
        let inner = service_fn(|_req: Request<()>| async {
            Ok::<_, std::convert::Infallible>(Response::new(()))
        });

        let mut svc = layer().layer(inner);
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(Request::new(()))
            .await
            .unwrap();

        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
        assert_eq!(
            response.headers().get("referrer-policy").unwrap(),
            "no-referrer"
        );
        assert!(response.headers().contains_key("permissions-policy"));
    }
}
