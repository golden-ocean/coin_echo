//! Casbin 权限校验中间件：读取请求扩展里已由 `jwt.rs` 注入的
//! [`Claims`]，取 `sub` 作为 subject，结合请求路径/方法调用
//! `CasbinEnforcer::check`，未授权时返回 403。
//!
//! # 薄适配层原则
//!
//! 权限模型、策略存储全部在 `platform_security::casbin::CasbinEnforcer`
//! 里，本文件只做"从请求提取 subject/object/action → 调用 check →
//! 处理结果"。
//!
//! # 依赖顺序
//!
//! 必须应用在 [`super::jwt`] 之后（在 `Router::layer()` 调用顺序上更
//! 早——即比 jwt 更外层是错的，应该让 jwt 先执行、casbin 在它内层）：
//! casbin 需要读取请求扩展里的 `Claims`，这个扩展由 jwt 中间件注入，
//! 必须先经过身份认证才能做权限判断，顺序反了会导致 `Claims` 缺失。
//!
//! # object/action 的取值
//!
//! 默认用请求路径作为 `object`、HTTP 方法作为 `action`（如
//! `GET /v1/users/1` 对应 `object = "/v1/users/1"`,
//! `action = "GET"`）。这是最直接的映射方式，如果未来策略模型需要更
//! 粗粒度的 object（如按资源类型而非具体路径匹配，`/v1/users/*`），
//! 应该在 Casbin 的 matcher 表达式里用路径前缀匹配处理，不需要改这里
//! 的提取逻辑。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{HeaderValue, Request, Response, StatusCode, header};
use platform_kernel::error::{ErrorKind, ErrorMeta};
use platform_kernel::http::ProblemDetails;
use platform_security::casbin::CasbinEnforcer;
use platform_security::jwt::Claims;
use tower::{Layer, Service};

use crate::context::RequestContext;

#[derive(Clone)]
pub struct CasbinAuthLayer {
    enforcer: Arc<CasbinEnforcer>,
}

impl CasbinAuthLayer {
    #[must_use]
    pub fn new(enforcer: Arc<CasbinEnforcer>) -> Self {
        Self { enforcer }
    }
}

impl<S> Layer<S> for CasbinAuthLayer {
    type Service = CasbinAuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CasbinAuthMiddleware {
            inner,
            enforcer: Arc::clone(&self.enforcer),
        }
    }
}

#[derive(Clone)]
pub struct CasbinAuthMiddleware<S> {
    inner: S,
    enforcer: Arc<CasbinEnforcer>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CasbinAuthMiddleware<S>
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
        let enforcer = Arc::clone(&self.enforcer);
        let mut inner = self.inner.clone();

        let claims = request.extensions().get::<Claims>().cloned();
        let object = request.uri().path().to_string();
        let action = request.method().as_str().to_string();

        Box::pin(async move {
            // 没有 Claims 意味着 jwt 中间件没有正确挂在这层之外——这是
            // 配置错误（顺序反了），而非正常的"未认证"路径（那种情况
            // jwt 中间件已经在更外层拦截并返回 401 了，请求根本不会
            // 到达这里）。这里同样按 403 处理，不额外区分，避免把内部
            // 配置错误的细节暴露给客户端。
            let Some(claims) = claims else {
                return Ok(forbidden_response::<ResBody>());
            };

            match enforcer.check(&claims.sub, &object, &action).await {
                Ok(()) => inner.call(request).await,
                Err(_) => Ok(forbidden_response::<ResBody>()),
            }
        })
    }
}

fn forbidden_response<ResBody: From<Bytes>>() -> Response<ResBody> {
    let ctx = RequestContext::current_or_default();
    let problem = ProblemDetails::from_error(&CasbinAuthError, "app", ctx.instance, ctx.trace_id);
    let payload = serde_json::to_vec(&problem).unwrap_or_default();

    Response::builder()
        .status(StatusCode::FORBIDDEN)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )
        .body(ResBody::from(Bytes::from(payload)))
        .unwrap_or_else(|_| Response::new(ResBody::from(Bytes::new())))
}

struct CasbinAuthError;

impl ErrorMeta for CasbinAuthError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Forbidden
    }
    fn code(&self) -> &'static str {
        "security.permission_denied"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use std::io::Write;
    use tower::{ServiceExt, service_fn};

    fn temp_enforcer_config() -> (tempfile::TempDir, platform_security::casbin::CasbinConfig) {
        let dir = tempfile::tempdir().unwrap();

        let model_path = dir.path().join("model.conf");
        let mut model_file = std::fs::File::create(&model_path).unwrap();
        write!(
            model_file,
            "[request_definition]\nr = sub, obj, act\n\n\
             [policy_definition]\np = sub, obj, act\n\n\
             [policy_effect]\ne = some(where (p.eft == allow))\n\n\
             [matchers]\nm = r.sub == p.sub && r.obj == p.obj && r.act == p.act\n"
        )
        .unwrap();

        let policy_path = dir.path().join("policy.csv");
        let mut policy_file = std::fs::File::create(&policy_path).unwrap();
        writeln!(policy_file, "p, user-1, /v1/users, GET").unwrap();

        let config = platform_security::casbin::CasbinConfig {
            model_path: model_path.to_string_lossy().to_string(),
            policy_path: policy_path.to_string_lossy().to_string(),
        };
        (dir, config)
    }

    fn request_with_claims(sub: &str, path: &str, method: Method) -> Request<()> {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .body(())
            .unwrap();
        request.extensions_mut().insert(Claims {
            sub: sub.to_string(),
            iss: "app".to_string(),
            iat: 0,
            exp: i64::MAX,
        });
        request
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

    #[tokio::test]
    async fn allowed_request_passes_through() {
        let (_dir, config) = temp_enforcer_config();
        let enforcer = Arc::new(CasbinEnforcer::new(&config).await.unwrap());

        let mut svc = CasbinAuthLayer::new(enforcer).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_with_claims("user-1", "/v1/users", Method::GET))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn disallowed_request_returns_403() {
        let (_dir, config) = temp_enforcer_config();
        let enforcer = Arc::new(CasbinEnforcer::new(&config).await.unwrap());

        let mut svc = CasbinAuthLayer::new(enforcer).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_with_claims("user-1", "/v1/users", Method::DELETE))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn missing_claims_returns_403_not_panic() {
        let (_dir, config) = temp_enforcer_config();
        let enforcer = Arc::new(CasbinEnforcer::new(&config).await.unwrap());

        let mut svc = CasbinAuthLayer::new(enforcer).layer(inner_ok());
        let request = Request::builder()
            .method(Method::GET)
            .uri("/v1/users")
            .body(())
            .unwrap();
        let response = svc.ready().await.unwrap().call(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
