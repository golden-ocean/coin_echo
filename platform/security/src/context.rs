use std::borrow::Cow;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Response},
};
use platform_kernel::{
    error::{ErrorKind, ErrorMeta, FieldError},
    http::ProblemDetails,
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy)]
pub struct SecurityContext {
    /// 当前操作发起人的用户 ID
    user_id: Uuid,
    // /// 用户所属租户/组织 ID（多租户系统常用）
    // pub tenant_id: Option<String>,
    /// 标识当前上下文是否为系统内部特权任务（如 Cron Job, MQ 异步处理）
    is_system: bool,
}
impl SecurityContext {
    pub fn id(&self) -> Uuid {
        self.user_id
    }
    pub fn is_system(&self) -> bool {
        self.is_system
    }
}

impl SecurityContext {
    /// 构造普通的端上用户上下文
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            is_system: false,
        }
    }

    /// 构造系统内部任务上下文（如定时任务、MQ 消费者、内部初始化）
    pub fn system() -> Self {
        Self {
            user_id: Uuid::nil(),
            is_system: true,
        }
    }
}

impl<S> FromRequestParts<S> for SecurityContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<SecurityContext>()
            .copied()
            .ok_or_else(|| {
                // 这不是常规的"未登录"（那是 JwtAuthMiddleware 自己判断后返回的
                // 401），而是这条路由压根没被 JwtAuthLayer 保护——属于部署/路由
                // 配置错误，用 error 级别打日志方便被监控捕捉到，而不是当成
                // 普通的客户端错误悄悄吞掉。
                tracing::error!(
                    component = "SecurityContext",
                    event = "extension_missing",
                    "SecurityContext extension 缺失，该路由可能未挂载 JwtAuthLayer"
                );

                let instance = parts.uri.path().to_string();

                let trace_id = parts
                    .headers
                    .get("x-trace-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown-trace")
                    .to_string();
                let problem = ProblemDetails::from_error(
                    &SecurityContextError::Missing,
                    "platform_security",
                    instance,
                    trace_id,
                );
                (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(problem)).into_response()
            })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityContextError {
    #[error("请求扩展中缺失 SecurityContext，中间件可能未配置或顺序错误")]
    Missing,
}

impl ErrorMeta for SecurityContextError {
    fn kind(&self) -> ErrorKind {
        match self {
            SecurityContextError::Missing => ErrorKind::Internal,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            SecurityContextError::Missing => "platform_security.context.missing",
        }
    }

    fn detail(&self) -> Option<Cow<'_, str>> {
        match self {
            SecurityContextError::Missing => Some(Cow::Borrowed(
                "未找到有效的认证上下文，请确保已提供正确的 Auth Token",
            )),
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        Vec::new()
    }
}
