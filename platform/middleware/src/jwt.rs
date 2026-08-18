//! JWT 鉴权中间件：从 `Authorization: Bearer <token>` 提取并校验令牌，
//! 通过后把 [`Claims`] 存入请求扩展，供 handler 用 `Extension<Claims>`
//! 取用。
//!
//! # 薄适配层原则
//!
//! 本文件**不实现**任何 JWT 编解码逻辑——签发/校验的核心逻辑在
//! `platform_security::jwt::JwtCodec` 里，是纯逻辑、零 tower/axum 依赖，
//! 可以被不认识 HTTP 的业务代码（如 `iam-application` 签发登录令牌）
//! 直接复用。本文件只做三件事：从请求头取出 token 字符串、调用
//! `JwtCodec::verify_access`、把结果放进请求扩展或构造错误响应。
//!
//! # 手写为 `tower::Layer` + `tower::Service`
//!
//! 不用 `axum::middleware::from_fn`：这样它是纯 tower 组件，不绑定
//! axum 的 handler 签名约定，理论上可以搬到任何基于 tower 的服务栈。
//!
//! # 不进全局中间件栈
//!
//! 不是所有路由都需要鉴权，由各领域 `xxx-api` 在需要保护的子路由上
//! 显式 `.layer(JwtAuthLayer::new(jwt_codec))`，见 `apply.rs` 的文档
//! 说明——`apply.rs` 只组装全局默认生效的中间件，鉴权类中间件属于
//! 路由级别的可选组件。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{HeaderValue, Request, Response, StatusCode, header};
use platform_kernel::error::{ErrorKind, ErrorMeta};
use platform_kernel::http::ProblemDetails;
use platform_security::jwt::{Claims, JwtCodec};
use tower::{Layer, Service};
use uuid::Uuid;

use crate::context::RequestContext;

/// 当前请求的认证身份，中间件校验通过后写入请求扩展。
/// 任何业务 crate 的 handler 都可以直接 `Extension<CurrentUser>` 取用，
/// 不需要认识 Claims/JwtCodec 这些 JWT 实现细节，也不需要依赖 iam 或
/// 任何其他具体业务 crate。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentUser {
    pub id: Uuid,
}

impl CurrentUser {
    pub(crate) fn new(id: Uuid) -> Self {
        Self { id }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(Clone)]
pub struct JwtAuthLayer {
    codec: Arc<JwtCodec>,
}

impl JwtAuthLayer {
    #[must_use]
    pub fn new(codec: Arc<JwtCodec>) -> Self {
        Self { codec }
    }
}

impl<S> Layer<S> for JwtAuthLayer {
    type Service = JwtAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        JwtAuthMiddleware {
            inner,
            codec: Arc::clone(&self.codec),
        }
    }
}

#[derive(Clone)]
pub struct JwtAuthMiddleware<S> {
    inner: S,
    codec: Arc<JwtCodec>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for JwtAuthMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: From<Bytes> + Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response<ResBody>, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let codec = Arc::clone(&self.codec);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let token = extract_bearer_token(&request);
            let claims: Option<Claims> = token.and_then(|t| codec.verify_access(t).ok());

            let Some(claims) = claims else {
                return Ok(unauthorized_response::<ResBody>());
            };
            // sub 解析失败按无效令牌处理，和签名/声明校验失败走同一条 401 路径，
            // 不给调用方多一种可以探测的错误形态。
            let Ok(user_id) = Uuid::parse_str(&claims.sub) else {
                return Ok(unauthorized_response::<ResBody>());
            };

            let mut request = request;
            request.extensions_mut().insert(claims);
            request.extensions_mut().insert(CurrentUser::new(user_id));
            inner.call(request).await
        })
    }
}

fn extract_bearer_token<ReqBody>(request: &Request<ReqBody>) -> Option<&str> {
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

fn unauthorized_response<ResBody: From<Bytes>>() -> Response<ResBody> {
    let ctx = RequestContext::current_or_default();
    let problem = ProblemDetails::from_error(&JwtError, "app", ctx.instance, ctx.trace_id);
    let payload = serde_json::to_vec(&problem).unwrap_or_default();

    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )
        .header(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"))
        .body(ResBody::from(Bytes::from(payload)))
        .unwrap_or_else(|_| Response::new(ResBody::from(Bytes::new())))
}

struct JwtError;

impl ErrorMeta for JwtError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Unauthenticated
    }
    fn code(&self) -> &'static str {
        "security.missing_or_invalid_token"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use platform_kernel::time::SystemClock;
    use platform_security::jwt::JwtConfig;
    use tower::{ServiceExt, service_fn};

    fn codec() -> Arc<JwtCodec> {
        let config = JwtConfig {
            access_secret: "a".repeat(32),
            access_expire_minutes: 15,
            refresh_secret: "b".repeat(32),
            refresh_expire_hours: 720,
            issuer: "app".to_string(),
        };
        Arc::new(JwtCodec::new(&config, Arc::new(SystemClock)))
    }

    fn inner_ok() -> impl Service<
        Request<()>,
        Response = Response<Bytes>,
        Error = std::convert::Infallible,
        Future = impl Future<Output = Result<Response<Bytes>, std::convert::Infallible>> + Send,
    > + Clone {
        service_fn(|_req: Request<()>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Bytes::from_static(b"ok")))
        })
    }

    fn request(auth_header: Option<&str>) -> Request<()> {
        let mut builder = Request::builder().method(Method::GET).uri("/protected");
        if let Some(h) = auth_header {
            builder = builder.header(header::AUTHORIZATION, h);
        }
        builder.body(()).unwrap()
    }

    #[tokio::test]
    async fn missing_token_returns_401() {
        let mut svc = JwtAuthLayer::new(codec()).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request(None))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_token_passes_through_and_injects_claims() {
        let codec = codec();
        let user_id = Uuid::new_v4();
        let pair = codec.issue(&user_id.to_string()).unwrap();

        let inner = service_fn(move |req: Request<()>| async move {
            // 验证 claims 确实被注入了请求扩展。
            let claims = req.extensions().get::<Claims>().cloned();
            assert_eq!(claims.map(|c| c.sub), Some(user_id.to_string()));
            Ok::<_, std::convert::Infallible>(Response::new(Bytes::from_static(b"ok")))
        });

        let mut svc = JwtAuthLayer::new(Arc::clone(&codec)).layer(inner);
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request(Some(&format!("Bearer {}", pair.access_token))))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn malformed_header_returns_401() {
        let mut svc = JwtAuthLayer::new(codec()).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request(Some("NotBearer xyz")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn expired_or_forged_token_returns_401() {
        let mut svc = JwtAuthLayer::new(codec()).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request(Some("Bearer not-a-real-jwt")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_token_injects_current_user_with_parsed_uuid() {
        let codec = codec();
        let user_id = Uuid::new_v4();
        let pair = codec.issue(&user_id.to_string()).unwrap();

        let inner = service_fn(move |req: Request<()>| async move {
            let current_user = req.extensions().get::<CurrentUser>().copied();
            assert_eq!(current_user.map(|u| u.id()), Some(user_id));
            Ok::<_, std::convert::Infallible>(Response::new(Bytes::from_static(b"ok")))
        });

        let mut svc = JwtAuthLayer::new(Arc::clone(&codec)).layer(inner);
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request(Some(&format!("Bearer {}", pair.access_token))))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn sub_not_a_valid_uuid_rejected_as_unauthorized() {
        // claims.sub 是任意字符串，JWT 协议本身不保证它是合法 UUID，
        // 这里显式验证"格式不对"和"签名/过期"走的是同一条 401 路径。
        let codec = codec();
        let pair = codec.issue("not-a-uuid").unwrap();

        let mut svc = JwtAuthLayer::new(codec).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request(Some(&format!("Bearer {}", pair.access_token))))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
