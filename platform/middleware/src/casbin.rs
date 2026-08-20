//! Casbin 鉴权中间件：读取请求扩展中的 [`SecurityContext`]（由
//! [`super::jwt::JwtAuthLayer`] 更早的中间件写入），按调用方声明的
//! 权限码调用 `CasbinEnforcer::check`，决定是否放行。
//!
//! # 薄适配层原则
//!
//! 本文件**不实现**任何策略判定逻辑——`allow`/`deny` 的核心判断在
//! `platform_security::casbin::CasbinEnforcer` 里，是纯逻辑组件，不
//! 认识 HTTP/tower。本文件只做三件事：从请求扩展取出
//! [`SecurityContext`]、调用 `CasbinEnforcer::check`、把结果转换成
//! HTTP 响应或放行请求。
//!
//! # 手写为 `tower::Layer` + `tower::Service`
//!
//! 和 [`super::jwt::JwtAuthLayer`] 同样的理由：保持纯 tower 组件，不
//! 绑定 axum 的 handler 签名约定。
//!
//! # 权限码在构造时静态声明，不做运行时反查
//!
//! `permission_code` 是构造 [`PermissionLayer`] 时传入的
//! `&'static str`，在“注册路由”这一步就已经确定——中间件本身不知道
//! 也不需要知道当前请求匹配的是哪个路由，调用方（各 `xxx-api` 的路由
//! 表）负责把正确的权限码和正确的路由绑在一起，通常通过
//! `.route_layer(PermissionLayer::new(enforcer, "iam:role:add"))`
//! 逐条路由声明。
//!
//! # 必须挂在 [`super::jwt::JwtAuthLayer`] 之后（更内层）
//!
//! 本中间件依赖请求扩展里已经存在 [`SecurityContext`]，如果
//! `JwtAuthLayer` 没有先执行过，这里会读不到，按“配置错误”处理并返回
//! 5xx（不是普通的“未登录” 401——那是路由挂载顺序错误，属于部署配置
//! 问题，要能被监控捕捉到，不能悄悄当成客户端错误吞掉）。
//!
//! # 系统内部任务直接放行
//!
//! [`SecurityContext::system`] 构造的上下文（定时任务/MQ 消费者/内部
//! 初始化等场景）不经过 Casbin 检查，直接放行——这类任务没有“当前
//! 登录用户”的概念，走策略引擎判断没有意义。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::Bytes;
use http::{HeaderValue, Request, Response, StatusCode, header};
use platform_kernel::error::ErrorMeta;
use platform_kernel::http::ProblemDetails;
use platform_security::casbin::{CasbinEnforcer, CasbinError};
use platform_security::context::{SecurityContext, SecurityContextError};
use tower::{Layer, Service};

use crate::context::RequestContext;

#[derive(Clone)]
pub struct PermissionLayer {
    enforcer: Arc<CasbinEnforcer>,
    permission_code: &'static str,
}

impl PermissionLayer {
    #[must_use]
    pub fn new(enforcer: Arc<CasbinEnforcer>, permission_code: &'static str) -> Self {
        Self {
            enforcer,
            permission_code,
        }
    }
}

impl<S> Layer<S> for PermissionLayer {
    type Service = PermissionMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        PermissionMiddleware {
            inner,
            enforcer: Arc::clone(&self.enforcer),
            permission_code: self.permission_code,
        }
    }
}

#[derive(Clone)]
pub struct PermissionMiddleware<S> {
    inner: S,
    enforcer: Arc<CasbinEnforcer>,
    permission_code: &'static str,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for PermissionMiddleware<S>
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
        let permission_code = self.permission_code;
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let ctx = request.extensions().get::<SecurityContext>().copied();

            let Some(ctx) = ctx else {
                // 不是常规鉴权失败，是这条路由压根没被 JwtAuthLayer 保护——
                // 属于部署/路由配置错误，用 error 级别打日志方便被监控捕捉。
                tracing::error!(
                    component = "PermissionLayer",
                    permission_code,
                    event = "security_context_missing",
                    "SecurityContext 缺失，该路由可能未挂载 JwtAuthLayer 或顺序错误"
                );
                return Ok(problem_response::<ResBody>(&SecurityContextError::Missing));
            };

            // 系统内部任务（定时任务/MQ 消费者/内部初始化）不走策略检查
            if ctx.is_system() {
                return inner.call(request).await;
            }

            match enforcer.check(&ctx.id().to_string(), permission_code).await {
                Ok(()) => inner.call(request).await,
                Err(err @ CasbinError::PermissionDenied) => Ok(problem_response::<ResBody>(&err)),
                Err(err) => {
                    // InitFailed / PolicyOperationFailed 是基础设施故障，
                    // 不是"这个用户没权限"，单独打日志，方便和真实的权限拒绝区分开。
                    tracing::error!(
                        component = "PermissionLayer",
                        permission_code,
                        error = ?err,
                        "casbin 策略检查失败（非权限拒绝，属于基础设施错误）"
                    );
                    Ok(problem_response::<ResBody>(&err))
                }
            }
        })
    }
}

/// 把任意 [`ErrorMeta`] 错误转换成完整的 HTTP 响应，`status` 从
/// `ProblemDetails` 内部按 `kind()` 计算得出，不在这里写死状态码——
/// 这样 `CasbinError::PermissionDenied`（403）和
/// `CasbinError::InitFailed`/`PolicyOperationFailed`（500）能各自映射到
/// 正确的状态码，不会被一刀切成同一个值。
fn problem_response<ResBody: From<Bytes>>(err: &dyn ErrorMeta) -> Response<ResBody> {
    let ctx = RequestContext::current_or_default();
    let problem = ProblemDetails::from_error(err, "iam", ctx.instance, ctx.trace_id);
    let status = StatusCode::from_u16(problem.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let payload = serde_json::to_vec(&problem).unwrap_or_default();

    Response::builder()
        .status(status)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )
        .body(ResBody::from(Bytes::from(payload)))
        .unwrap_or_else(|_| Response::new(ResBody::from(Bytes::new())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use casbin::{Adapter, Model, Result as CasbinResult};
    use http::Method;
    use tower::{ServiceExt, service_fn};
    use uuid::Uuid;

    /// 测试专用的内存 adapter：构造时直接把要预置的 g/p 策略行塞进去，
    /// `load_policy` 时一次性写入 model，不需要真实存储。
    ///
    /// 注意：`casbin::Adapter` 的具体方法签名依 casbin crate 版本可能
    /// 略有出入，如果编译报错，对照项目实际引入的 casbin 版本调整。
    struct TestAdapter {
        lines: Vec<(&'static str, &'static str, Vec<String>)>,
    }

    #[async_trait::async_trait]
    impl Adapter for TestAdapter {
        async fn load_policy(&mut self, m: &mut dyn Model) -> CasbinResult<()> {
            for (sec, ptype, rule) in &self.lines {
                m.add_policy(sec, ptype, rule.clone());
            }
            Ok(())
        }

        async fn load_filtered_policy<'a>(
            &mut self,
            m: &mut dyn Model,
            _f: casbin::Filter<'a>,
        ) -> CasbinResult<()> {
            // 测试场景不需要真正的过滤加载语义，直接退化成全量加载即可，
            // 反正测试里策略条目本来就很少，不存在需要分片过滤的场景。
            self.load_policy(m).await
        }

        async fn save_policy(&mut self, _m: &mut dyn Model) -> CasbinResult<()> {
            Ok(())
        }
        async fn clear_policy(&mut self) -> CasbinResult<()> {
            Ok(())
        }
        fn is_filtered(&self) -> bool {
            false
        }
        async fn add_policy(
            &mut self,
            _sec: &str,
            _ptype: &str,
            _rule: Vec<String>,
        ) -> CasbinResult<bool> {
            Ok(true)
        }
        async fn add_policies(
            &mut self,
            _sec: &str,
            _ptype: &str,
            _rules: Vec<Vec<String>>,
        ) -> CasbinResult<bool> {
            Ok(true)
        }
        async fn remove_policy(
            &mut self,
            _sec: &str,
            _ptype: &str,
            _rule: Vec<String>,
        ) -> CasbinResult<bool> {
            Ok(true)
        }
        async fn remove_policies(
            &mut self,
            _sec: &str,
            _ptype: &str,
            _rules: Vec<Vec<String>>,
        ) -> CasbinResult<bool> {
            Ok(true)
        }
        async fn remove_filtered_policy(
            &mut self,
            _sec: &str,
            _ptype: &str,
            _field_index: usize,
            _field_values: Vec<String>,
        ) -> CasbinResult<bool> {
            Ok(true)
        }
    }

    async fn enforcer_with_policy(user_id: Uuid, permission_code: &str) -> Arc<CasbinEnforcer> {
        let adapter = TestAdapter {
            lines: vec![
                // g: user -> role
                ("g", "g", vec![user_id.to_string(), "tester".to_string()]),
                // p: role -> permission_code
                (
                    "p",
                    "p",
                    vec!["tester".to_string(), permission_code.to_string()],
                ),
            ],
        };
        Arc::new(CasbinEnforcer::with_adapter(adapter).await.unwrap())
    }

    async fn enforcer_without_policy() -> Arc<CasbinEnforcer> {
        Arc::new(
            CasbinEnforcer::with_adapter(TestAdapter { lines: vec![] })
                .await
                .unwrap(),
        )
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

    fn request_with_context(ctx: Option<SecurityContext>) -> Request<()> {
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/protected")
            .body(())
            .unwrap();
        if let Some(ctx) = ctx {
            request.extensions_mut().insert(ctx);
        }
        request
    }

    #[tokio::test]
    async fn missing_security_context_is_rejected() {
        let enforcer = enforcer_without_policy().await;
        let mut svc = PermissionLayer::new(enforcer, "iam:role:add").layer(inner_ok());

        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_with_context(None))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn system_context_bypasses_policy_check() {
        let enforcer = enforcer_without_policy().await; // 即使没有任何策略
        let mut svc = PermissionLayer::new(enforcer, "iam:role:add").layer(inner_ok());

        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_with_context(Some(SecurityContext::system())))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn user_with_matching_policy_is_allowed() {
        let user_id = Uuid::new_v4();
        let enforcer = enforcer_with_policy(user_id, "iam:role:add").await;
        let mut svc = PermissionLayer::new(enforcer, "iam:role:add").layer(inner_ok());

        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_with_context(Some(SecurityContext::new(user_id))))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn user_without_matching_policy_is_forbidden() {
        let user_id = Uuid::new_v4();
        // 策略里只允许 iam:role:add，但请求要求 iam:role:delete
        let enforcer = enforcer_with_policy(user_id, "iam:role:add").await;
        let mut svc = PermissionLayer::new(enforcer, "iam:role:delete").layer(inner_ok());

        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_with_context(Some(SecurityContext::new(user_id))))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
