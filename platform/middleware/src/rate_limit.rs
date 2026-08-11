//! 限流中间件：按客户端 IP 做固定窗口限流（自写 tower 中间件，不依赖
//! 任何第三方限流库）。
//!
//! # 为什么自己实现，而不用 `governor`/`tower_governor`
//!
//! 项目里评估过 `tower_governor`，但其 `GovernorLayer<K, M, RespBody>`
//! 的泛型签名、`error_handler` 字段在不同小版本间多次出现破坏性变更，
//! 排查成本明显高于自己实现一个固定窗口限流器。固定窗口算法虽然存在
//! 已知的"窗口边界双倍突发"局限（不如令牌桶/滑动窗口精确），但对当前
//! 阶段的防刷场景（保护接口不被单个来源过量调用）完全够用，且完全
//! 掌控在自己手里，不受外部库版本变化影响。
//!
//! # 设计
//!
//! - **按客户端 IP 分片**：每个 IP 拥有独立的固定窗口配额，用
//!   [`dashmap::DashMap`] 存储（分片锁，高并发下不会像单个
//!   `Mutex<HashMap>` 那样全局串行化）。一个恶意 IP 打满自己的配额
//!   不会影响其他客户端——这与"全局共享一份配额"的方案有本质区别。
//! - **固定窗口 + CAS 自旋**：单个 IP 内部的计数用 `AtomicU64` 无锁
//!   递减，窗口过期时用 CAS 重置，不使用 `Mutex`。
//! - **后台清理**：`DashMap` 里的条目只增不减会导致内存随着不同来源
//!   IP 数量无限增长（尤其面对爬虫、端口扫描等短时间大量不同 IP 的
//!   场景）。[`RateLimitLayer::new`] 内部启动一个周期性 `tokio::spawn`
//!   任务，清理超过若干个窗口周期未访问的条目。
//! - **`Retry-After` 精确计算**：不是固定返回整个窗口长度——窗口起点
//!   由第一次请求触发、时刻不确定，客户端被拒绝的时刻可能已经过了
//!   半个窗口，此时告诉它"再等一整个窗口"会让它多等不必要的时间。
//!
//! # 为什么依赖 `axum::body::Body`
//!
//! 这个中间件最终要挂载到 axum 路由链上，axum 内部路由响应体固定类型
//! 是 `axum::body::Body`。这是本 crate 中少数几个必须依赖 axum 的例外
//! （另一个是 `catch_panic.rs`），原因不是设计疏忽，而是这一层组件的
//! 本质就是 axum 专属的桥接点。
//!
//! # 应用位置
//!
//! 见 `apply.rs` 顺序说明：必须在 [`super::context::RequestContextLayer`]
//! 之内（更晚执行），因为构造 429 响应体时需要读取
//! `RequestContext::current()` 获取 `instance`/`trace_id`。

use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::ConnectInfo;
use dashmap::DashMap;
use http::{HeaderValue, Request, Response, StatusCode, header};
use platform_kernel::error::{ErrorKind, ErrorMeta};
use platform_kernel::http::ProblemDetails;
use tower::{Layer, Service};

use crate::context::RequestContext;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RateLimitConfig {
    /// 每个 IP 每个窗口内允许的最大请求数。
    #[serde(default = "RateLimitConfig::default_max_requests")]
    pub max_requests: u64,
    /// 窗口长度（秒）。
    #[serde(default = "RateLimitConfig::default_window_secs")]
    pub window_secs: u64,
}

impl RateLimitConfig {
    const fn default_max_requests() -> u64 {
        100
    }
    const fn default_window_secs() -> u64 {
        1
    }

    fn window(&self) -> Duration {
        Duration::from_secs(self.window_secs.max(1))
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: Self::default_max_requests(),
            window_secs: Self::default_window_secs(),
        }
    }
}

/// 单个 key（IP）的固定窗口限流状态（无锁）。
#[derive(Debug)]
struct WindowState {
    /// 窗口起始时间（相对 [`now_ns`] 基准的纳秒数）。
    window_start_ns: AtomicU64,
    /// 剩余配额。
    remaining: AtomicU64,
    /// 最近一次访问时间（纳秒），仅供后台清理任务判断是否为过期条目
    /// 使用，不参与限流判断本身。
    last_access_ns: AtomicU64,
    max: u64,
    window_ns: u64,
}

impl WindowState {
    fn new(max: u64, window_ns: u64) -> Self {
        let now = now_ns();
        Self {
            window_start_ns: AtomicU64::new(now),
            remaining: AtomicU64::new(max),
            last_access_ns: AtomicU64::new(now),
            max,
            window_ns,
        }
    }

    /// 无锁尝试消耗一个配额。窗口过期则 CAS 重置。
    fn try_acquire(&self) -> bool {
        self.last_access_ns.store(now_ns(), Ordering::Relaxed);
        loop {
            let rem = self.remaining.load(Ordering::Relaxed);
            if rem == 0 {
                let start = self.window_start_ns.load(Ordering::Relaxed);
                let elapsed = now_ns().saturating_sub(start);
                if elapsed >= self.window_ns {
                    let now = now_ns();
                    if self
                        .window_start_ns
                        .compare_exchange(start, now, Ordering::AcqRel, Ordering::Relaxed)
                        .is_ok()
                    {
                        self.remaining.store(self.max, Ordering::Release);
                    }
                    continue; // 无论 CAS 是否成功，重试
                }
                return false; // 窗口未过期，无配额
            }
            if self
                .remaining
                .compare_exchange(rem, rem - 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
            // CAS 失败，自旋重试
        }
    }

    /// 距当前窗口重置还需等待的秒数（向上取整），供 `Retry-After` 使用。
    fn retry_after_secs(&self) -> u64 {
        let start = self.window_start_ns.load(Ordering::Relaxed);
        let elapsed_ns = now_ns().saturating_sub(start);
        let remaining_ns = self.window_ns.saturating_sub(elapsed_ns);
        remaining_ns.div_ceil(1_000_000_000)
    }

    /// 距最近一次访问是否已超过 `stale_after`，供后台清理任务判断。
    fn is_stale(&self, stale_after_ns: u64) -> bool {
        let last = self.last_access_ns.load(Ordering::Relaxed);
        now_ns().saturating_sub(last) >= stale_after_ns
    }
}

fn now_ns() -> u64 {
    use std::sync::OnceLock;
    static BASE: OnceLock<Instant> = OnceLock::new();
    let base = BASE.get_or_init(Instant::now);
    base.elapsed().as_nanos() as u64
}

/// 按 IP 分片的限流状态存储 + 后台清理任务的句柄。
struct RateLimitStore {
    states: DashMap<IpAddr, WindowState>,
    max: u64,
    window_ns: u64,
}

impl RateLimitStore {
    fn try_acquire(&self, ip: IpAddr) -> (bool, u64) {
        let entry = self
            .states
            .entry(ip)
            .or_insert_with(|| WindowState::new(self.max, self.window_ns));
        let allowed = entry.try_acquire();
        let retry_after = if allowed { 0 } else { entry.retry_after_secs() };
        (allowed, retry_after)
    }
}

/// 限流层。
#[derive(Clone)]
pub struct RateLimitLayer {
    store: Arc<RateLimitStore>,
}

impl RateLimitLayer {
    /// 构造限流层，并启动后台清理任务。
    ///
    /// 清理周期与"判定为过期"的阈值：每隔 `10 * window` 扫描一次，
    /// 清除超过 `10 * window` 未访问的 IP 条目——阈值取窗口的整数倍，
    /// 既保证不会误删仍活跃的 IP（正常客户端至少每个窗口访问一次），
    /// 又不会因为扫描太频繁而增加额外开销。
    #[must_use]
    pub fn new(config: &RateLimitConfig) -> Self {
        assert!(config.max_requests > 0, "max_requests 必须大于 0");
        let window = config.window();
        assert!(!window.is_zero(), "window_secs 必须大于 0");

        let store = Arc::new(RateLimitStore {
            states: DashMap::new(),
            max: config.max_requests,
            window_ns: window.as_nanos() as u64,
        });

        spawn_cleanup_task(Arc::clone(&store), window);

        Self { store }
    }
}

/// `apply.rs` 里推荐的挂载方式：`RateLimitLayer::new(&config.rate_limit)`。
/// 保留这个薄函数是为了和其他中间件文件保持统一的 `xxx::layer(&config)`
/// 调用风格。
pub fn layer(config: &RateLimitConfig) -> RateLimitLayer {
    RateLimitLayer::new(config)
}

fn spawn_cleanup_task(store: Arc<RateLimitStore>, window: Duration) {
    let cleanup_interval = window * 10;
    let stale_after_ns = (window * 10).as_nanos() as u64;

    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(cleanup_interval);
        loop {
            ticker.tick().await;
            let before = store.states.len();
            store
                .states
                .retain(|_ip, state| !state.is_stale(stale_after_ns));
            let removed = before.saturating_sub(store.states.len());
            if removed > 0 {
                tracing::debug!(removed, remaining = store.states.len(), "限流状态清理完成");
            }
        }
    });
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            store: Arc::clone(&self.store),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    store: Arc<RateLimitStore>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for RateLimitService<S>
where
    S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, S::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let ip = extract_client_ip(&request);
        let store = Arc::clone(&self.store);
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let Some(ip) = ip else {
                // 拿不到客户端 IP（缺少 ConnectInfo 扩展）时放行而非
                // 拒绝——这是基础设施配置问题（忘记接
                // into_make_service_with_connect_info），不应该表现为
                // "所有请求都被限流"这种更隐蔽的故障模式。
                tracing::warn!("无法提取客户端 IP，跳过限流检查");
                return inner.call(request).await;
            };

            let (allowed, retry_after_secs) = store.try_acquire(ip);
            if allowed {
                inner.call(request).await
            } else {
                tracing::warn!(%ip, retry_after_secs, "请求被限流");
                Ok(too_many_requests_response(retry_after_secs))
            }
        })
    }
}

/// 从请求中提取客户端 IP。依赖 `axum::extract::ConnectInfo<SocketAddr>`
/// 扩展——`apps/server` 启动服务时需用
/// `into_make_service_with_connect_info::<SocketAddr>()`，保证该扩展
/// 总是存在；拿不到时说明基础设施配置有问题（而非正常业务场景）。
fn extract_client_ip<ReqBody>(request: &Request<ReqBody>) -> Option<IpAddr> {
    request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip())
}

fn too_many_requests_response(retry_after_secs: u64) -> Response<Body> {
    let ctx = RequestContext::current_or_default();
    let problem = ProblemDetails::from_error(&RateLimitedError, "app", ctx.instance, ctx.trace_id);
    let payload = serde_json::to_vec(&problem).unwrap_or_default();

    Response::builder()
        .status(StatusCode::TOO_MANY_REQUESTS)
        .header(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )
        .header(header::RETRY_AFTER, HeaderValue::from(retry_after_secs))
        .body(Body::from(payload))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

struct RateLimitedError;

impl ErrorMeta for RateLimitedError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Exhausted
    }
    fn code(&self) -> &'static str {
        "security.rate_limited"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Method;
    use tower::{ServiceExt, service_fn};

    fn config(max_requests: u64, window_secs: u64) -> RateLimitConfig {
        RateLimitConfig {
            max_requests,
            window_secs,
        }
    }

    fn request_from(ip: &str) -> Request<()> {
        let addr: SocketAddr = format!("{ip}:12345").parse().unwrap();
        let mut request = Request::builder()
            .method(Method::GET)
            .uri("/x")
            .body(())
            .unwrap();
        request.extensions_mut().insert(ConnectInfo(addr));
        request
    }

    fn request_without_connect_info() -> Request<()> {
        Request::builder()
            .method(Method::GET)
            .uri("/x")
            .body(())
            .unwrap()
    }

    fn inner_ok() -> impl Service<
        Request<()>,
        Response = Response<Body>,
        Error = std::convert::Infallible,
        Future = impl Future<Output = Result<Response<Body>, std::convert::Infallible>> + Send,
    > + Clone {
        service_fn(|_req: Request<()>| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::from("ok")))
        })
    }

    #[test]
    fn window_state_exhausts_and_resets_after_expiry() {
        let state = WindowState::new(2, Duration::from_millis(50).as_nanos() as u64);
        assert!(state.try_acquire());
        assert!(state.try_acquire());
        assert!(!state.try_acquire());

        std::thread::sleep(Duration::from_millis(51));
        assert!(state.try_acquire());
    }

    #[test]
    fn retry_after_secs_decreases_as_window_elapses() {
        let state = WindowState::new(1, Duration::from_secs(5).as_nanos() as u64);
        assert!(state.try_acquire());

        let first = state.retry_after_secs();
        std::thread::sleep(Duration::from_millis(1100));
        let second = state.retry_after_secs();

        assert!(
            second < first,
            "距重置的剩余时间应随等待而减少：{first} -> {second}"
        );
    }

    #[tokio::test]
    async fn requests_within_quota_pass_through() {
        let mut svc = RateLimitLayer::new(&config(5, 1)).layer(inner_ok());
        for _ in 0..5 {
            let response = svc
                .ready()
                .await
                .unwrap()
                .call(request_from("1.2.3.4"))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn request_exceeding_quota_is_rejected_with_429_and_retry_after() {
        let mut svc = RateLimitLayer::new(&config(2, 5)).layer(inner_ok());
        for _ in 0..2 {
            svc.ready()
                .await
                .unwrap()
                .call(request_from("5.6.7.8"))
                .await
                .unwrap();
        }
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("5.6.7.8"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let retry_after: u64 = response
            .headers()
            .get(header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .expect("应包含合法的 Retry-After 头");
        assert!(retry_after > 0 && retry_after <= 5);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            "application/problem+json"
        );
    }

    #[tokio::test]
    async fn different_ips_have_independent_quotas() {
        let mut svc = RateLimitLayer::new(&config(1, 5)).layer(inner_ok());

        let r1 = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("9.9.9.9"))
            .await
            .unwrap();
        assert_eq!(r1.status(), StatusCode::OK);
        let r1_blocked = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("9.9.9.9"))
            .await
            .unwrap();
        assert_eq!(r1_blocked.status(), StatusCode::TOO_MANY_REQUESTS);

        let r2 = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("8.8.8.8"))
            .await
            .unwrap();
        assert_eq!(r2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn missing_connect_info_falls_back_to_passthrough() {
        let mut svc = RateLimitLayer::new(&config(1, 5)).layer(inner_ok());
        let response = svc
            .ready()
            .await
            .unwrap()
            .call(request_without_connect_info())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn quota_resets_after_window_expires() {
        let mut svc = RateLimitLayer::new(&config(1, 1)).layer(inner_ok());
        let ok = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("2.2.2.2"))
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);

        let blocked = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("2.2.2.2"))
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);

        tokio::time::sleep(Duration::from_millis(1100)).await;
        let ok_again = svc
            .ready()
            .await
            .unwrap()
            .call(request_from("2.2.2.2"))
            .await
            .unwrap();
        assert_eq!(ok_again.status(), StatusCode::OK);
    }
}
