// //! CSRF（跨站请求伪造）防护：signed double-submit cookie 模式。
// //!
// //! # 为什么需要这个中间件
// //!
// //! 项目鉴权方案是 HttpOnly Cookie（access + refresh）+ Silent Refresh——
// //! 浏览器会在同源及跨站请求中自动附带 Cookie，这正是 CSRF 攻击的前提
// //! （攻击者在别的站点构造一个指向本服务的请求，浏览器照样自动带上合法
// //! 的认证 Cookie）。HttpOnly 只防 XSS 窃取 token 内容，不防 CSRF——两者
// //! 是独立的攻击面，必须分别防护。若鉴权方案是纯 Bearer Token（前端手动
// //! 从 localStorage 读出塞进 `Authorization` 头），则不需要本中间件，
// //! 因为浏览器不会自动附带这个头。
// //!
// //! # 为什么选 signed double-submit cookie，而非服务端存储的 token
// //!
// //! 服务端存储方案（如 Redis 记录 token→用户 的映射）需要额外的存储
// //! 依赖和过期清理逻辑；double-submit 是无状态的：服务端只需要一个签名
// //! 密钥即可校验，不需要记住签发过哪些 token。这是 OWASP CSRF 防护
// //! 备忘录推荐的两种主流方案之一（另一种是 synchronizer token
// //! pattern，需要绑定 session，与本项目的无状态鉴权风格不符）。
// //!
// //! # 防护原理
// //!
// //! 1. 安全方法（`GET`/`HEAD`/`OPTIONS`）：放行；若请求未携带 CSRF
// //!    cookie，在响应里种一个新的（非 HttpOnly，前端 JS 需要读取它并
// //!    放进请求头）。
// //! 2. 不安全方法（`POST`/`PUT`/`PATCH`/`DELETE`）：要求请求同时携带
// //!    cookie 与自定义头 [`CsrfConfig::header_name`]，两者值必须完全
// //!    一致（double-submit），且 cookie 值的 HMAC 签名必须校验通过。
// //! 3. 攻击者能让浏览器自动带上 cookie，但受同源策略限制读不到 cookie
// //!    的值，因此无法构造出匹配的请求头——请求会在"header 与 cookie
// //!    不一致"这一步被拒绝。签名校验则进一步防御攻击者试图自己伪造一个
// //!    "看起来合法"的 cookie 值（如通过子域名 cookie 注入等旁路手段）。
// //!
// //! # 应用位置
// //!
// //! 必须在 [`super::jwt`] 之后（`Router::layer()` 调用顺序上更早，即
// //! 更外层）——CSRF 校验不依赖身份认证结果，是否需要认证是各路由自己
// //! 的事，CSRF 是所有可能被 Cookie 自动携带的状态变更请求都该过一遍的
// //! 通用防线。

// use std::future::Future;
// use std::pin::Pin;
// use std::task::{Context, Poll};

// use base64::Engine;
// use base64::engine::general_purpose::URL_SAFE_NO_PAD;
// use bytes::Bytes;
// use hmac::{Hmac, Mac};
// use http::{HeaderValue, Method, Request, Response, StatusCode, header};
// use platform_config::ConfigMeta;
// use platform_kernel::error::{ErrorKind, ErrorMeta};
// use platform_kernel::http::ProblemDetails;
// use rand::RngCore;
// use serde::Deserialize;
// use sha2::Sha256;
// use tower::{Layer, Service};

// type HmacSha256 = Hmac<Sha256>;

// /// CSRF 防护配置。对应环境变量前缀 `CSRF_`。
// #[derive(Debug, Clone, Deserialize)]
// pub struct CsrfConfig {
//     /// HMAC 签名密钥，必须与 JWT 密钥不同（不同用途的密钥不应共用，
//     /// 一处泄露不该连带影响另一处）。
//     pub secret: String,

//     #[serde(default = "CsrfConfig::default_cookie_name")]
//     pub cookie_name: String,

//     #[serde(default = "CsrfConfig::default_header_name")]
//     pub header_name: String,

//     /// Cookie 的 `Max-Age`（秒）。
//     #[serde(default = "CsrfConfig::default_max_age_secs")]
//     pub max_age_secs: u64,

//     /// 生产环境必须为 `true`（要求 HTTPS 才发送该 Cookie）；本地非
//     /// HTTPS 开发环境可设为 `false`。
//     #[serde(default = "CsrfConfig::default_secure")]
//     pub secure: bool,
// }

// #[derive(Debug, thiserror::Error)]
// pub enum CsrfConfigError {
//     #[error("secret 长度过短（{len} 字节），至少需要 {min} 字节")]
//     SecretTooShort { len: usize, min: usize },
// }

// impl CsrfConfig {
//     const MIN_SECRET_LEN: usize = 32;

//     fn default_cookie_name() -> String {
//         "csrf_token".to_string()
//     }
//     fn default_header_name() -> String {
//         "x-csrf-token".to_string()
//     }
//     const fn default_max_age_secs() -> u64 {
//         24 * 60 * 60 // 1 天
//     }
//     const fn default_secure() -> bool {
//         true
//     }
// }

// impl ConfigMeta for CsrfConfig {
//     type Error = CsrfConfigError;

//     fn prefix() -> &'static str {
//         "CSRF_"
//     }

//     fn validate(&self) -> Result<(), Self::Error> {
//         if self.secret.len() < Self::MIN_SECRET_LEN {
//             return Err(CsrfConfigError::SecretTooShort {
//                 len: self.secret.len(),
//                 min: Self::MIN_SECRET_LEN,
//             });
//         }
//         Ok(())
//     }
// }

// // ---- 签名 token 的生成与校验 ----

// /// 生成一个新的签名 token：`base64url(random) . base64url(hmac(random))`。
// fn generate_signed_token(secret: &str) -> String {
//     let mut random_bytes = [0u8; 24];
//     rand::thread_rng().fill_bytes(&mut random_bytes);
//     let random_part = URL_SAFE_NO_PAD.encode(random_bytes);

//     let signature = sign(secret, random_part.as_bytes());
//     format!("{random_part}.{signature}")
// }

// /// 校验 token 的签名是否与密钥匹配（不依赖服务端存储，纯计算校验）。
// fn verify_signed_token(secret: &str, token: &str) -> bool {
//     let Some((random_part, signature)) = token.split_once('.') else {
//         return false;
//     };
//     let expected = sign(secret, random_part.as_bytes());
//     // 用固定时间比较，避免时序攻击泄露签名的正确前缀长度。
//     constant_time_eq(expected.as_bytes(), signature.as_bytes())
// }

// fn sign(secret: &str, data: &[u8]) -> String {
//     let mut mac =
//         HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC 接受任意长度密钥，不应失败");
//     mac.update(data);
//     URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
// }

// fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
//     if a.len() != b.len() {
//         return false;
//     }
//     a.iter()
//         .zip(b.iter())
//         .fold(0u8, |acc, (x, y)| acc | (x ^ y))
//         == 0
// }

// // ---- Cookie 头的读取与构造（手动解析，避免引入完整 cookie 解析库） ----

// fn read_cookie<'a>(request: &'a Request<impl Sized>, name: &str) -> Option<&'a str> {
//     request
//         .headers()
//         .get(header::COOKIE)?
//         .to_str()
//         .ok()?
//         .split(';')
//         .filter_map(|pair| pair.trim().split_once('='))
//         .find(|(key, _)| *key == name)
//         .map(|(_, value)| value)
// }

// fn build_set_cookie_header(config: &CsrfConfig, value: &str) -> HeaderValue {
//     let mut cookie = format!(
//         "{}={}; Path=/; Max-Age={}; SameSite=Strict",
//         config.cookie_name, value, config.max_age_secs
//     );
//     if config.secure {
//         cookie.push_str("; Secure");
//     }
//     // 有意不设置 HttpOnly——前端 JS 必须能读取这个 cookie 的值，才能
//     // 把它放进请求头完成 double-submit 校验，这是本方案的核心前提。
//     HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static(""))
// }

// // ---- tower::Layer / Service ----

// #[derive(Clone)]
// pub struct CsrfLayer {
//     config: std::sync::Arc<CsrfConfig>,
// }

// impl CsrfLayer {
//     #[must_use]
//     pub fn new(config: CsrfConfig) -> Self {
//         Self {
//             config: std::sync::Arc::new(config),
//         }
//     }
// }

// impl<S> Layer<S> for CsrfLayer {
//     type Service = CsrfMiddleware<S>;

//     fn layer(&self, inner: S) -> Self::Service {
//         CsrfMiddleware {
//             inner,
//             config: std::sync::Arc::clone(&self.config),
//         }
//     }
// }

// #[derive(Clone)]
// pub struct CsrfMiddleware<S> {
//     inner: S,
//     config: std::sync::Arc<CsrfConfig>,
// }

// impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CsrfMiddleware<S>
// where
//     S: Service<Request<ReqBody>, Response = Response<ResBody>> + Clone + Send + 'static,
//     S::Future: Send + 'static,
//     ReqBody: Send + 'static,
//     ResBody: From<Bytes> + Send + 'static,
// {
//     type Response = Response<ResBody>;
//     type Error = S::Error;
//     type Future = Pin<Box<dyn Future<Output = Result<Response<ResBody>, S::Error>> + Send>>;

//     fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
//         self.inner.poll_ready(cx)
//     }

//     fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
//         let config = std::sync::Arc::clone(&self.config);
//         let mut inner = self.inner.clone();

//         let is_safe_method = matches!(
//             *request.method(),
//             Method::GET | Method::HEAD | Method::OPTIONS
//         );

//         if is_safe_method {
//             let existing_cookie = read_cookie(&request, &config.cookie_name).map(str::to_string);
//             return Box::pin(async move {
//                 let mut response = inner.call(request).await?;
//                 if existing_cookie.is_none() {
//                     let token = generate_signed_token(&config.secret);
//                     let cookie_header = build_set_cookie_header(&config, &token);
//                     response
//                         .headers_mut()
//                         .append(header::SET_COOKIE, cookie_header);
//                 }
//                 Ok(response)
//             });
//         }

//         // 不安全方法：cookie 与 header 必须同时存在、值一致、签名有效。
//         let cookie_value = read_cookie(&request, &config.cookie_name).map(str::to_string);
//         let header_value = request
//             .headers()
//             .get(config.header_name.as_str())
//             .and_then(|v| v.to_str().ok())
//             .map(str::to_string);

//         let valid = match (&cookie_value, &header_value) {
//             (Some(cookie), Some(header)) => {
//                 cookie == header && verify_signed_token(&config.secret, cookie)
//             }
//             _ => false,
//         };

//         if valid {
//             Box::pin(async move { inner.call(request).await })
//         } else {
//             Box::pin(async move { Ok(forbidden_response::<ResBody>()) })
//         }
//     }
// }

// fn forbidden_response<ResBody: From<Bytes>>() -> Response<ResBody> {
//     let problem = ProblemDetails::from_error(&CsrfError, "app", String::new(), String::new());
//     let payload = serde_json::to_vec(&problem).unwrap_or_default();

//     Response::builder()
//         .status(StatusCode::FORBIDDEN)
//         .header(
//             header::CONTENT_TYPE,
//             HeaderValue::from_static("application/problem+json"),
//         )
//         .body(ResBody::from(Bytes::from(payload)))
//         .unwrap_or_else(|_| Response::new(ResBody::from(Bytes::new())))
// }

// struct CsrfError;

// impl ErrorMeta for CsrfError {
//     fn kind(&self) -> ErrorKind {
//         ErrorKind::Forbidden
//     }
//     fn code(&self) -> &'static str {
//         "security.csrf_verification_failed"
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use tower::{ServiceExt, service_fn};

//     fn config() -> CsrfConfig {
//         CsrfConfig {
//             secret: "a".repeat(32),
//             cookie_name: "csrf_token".to_string(),
//             header_name: "x-csrf-token".to_string(),
//             max_age_secs: 3600,
//             secure: true,
//         }
//     }

//     // ---- token 签名/校验 ----

//     #[test]
//     fn generated_token_passes_its_own_verification() {
//         let secret = "a".repeat(32);
//         let token = generate_signed_token(&secret);
//         assert!(verify_signed_token(&secret, &token));
//     }

//     #[test]
//     fn tampered_token_fails_verification() {
//         let secret = "a".repeat(32);
//         let mut token = generate_signed_token(&secret);
//         token.push('x'); // 篡改签名部分
//         assert!(!verify_signed_token(&secret, &token));
//     }

//     #[test]
//     fn token_signed_with_different_secret_fails_verification() {
//         let token = generate_signed_token(&"a".repeat(32));
//         assert!(!verify_signed_token(&"b".repeat(32), &token));
//     }

//     #[test]
//     fn malformed_token_without_separator_fails_verification() {
//         assert!(!verify_signed_token(&"a".repeat(32), "not-a-valid-token"));
//     }

//     // ---- Service 行为 ----

//     fn inner_ok() -> impl Service<
//         Request<()>,
//         Response = Response<Bytes>,
//         Error = std::convert::Infallible,
//         Future = impl Future<Output = Result<Response<Bytes>, std::convert::Infallible>> + Send,
//     > + Clone {
//         service_fn(|_req: Request<()>| async {
//             Ok::<_, std::convert::Infallible>(Response::new(Bytes::from_static(b"ok")))
//         })
//     }

//     #[tokio::test]
//     async fn get_request_without_cookie_receives_new_csrf_cookie() {
//         let mut svc = CsrfLayer::new(config()).layer(inner_ok());
//         let request = Request::builder()
//             .method(Method::GET)
//             .uri("/x")
//             .body(())
//             .unwrap();

//         let response = svc.ready().await.unwrap().call(request).await.unwrap();
//         assert_eq!(response.status(), StatusCode::OK);
//         assert!(response.headers().get(header::SET_COOKIE).is_some());
//     }

//     #[tokio::test]
//     async fn get_request_with_existing_cookie_does_not_reissue() {
//         let mut svc = CsrfLayer::new(config()).layer(inner_ok());
//         let request = Request::builder()
//             .method(Method::GET)
//             .uri("/x")
//             .header(header::COOKIE, "csrf_token=already-set")
//             .body(())
//             .unwrap();

//         let response = svc.ready().await.unwrap().call(request).await.unwrap();
//         assert!(response.headers().get(header::SET_COOKIE).is_none());
//     }

//     #[tokio::test]
//     async fn post_without_csrf_cookie_or_header_is_rejected() {
//         let mut svc = CsrfLayer::new(config()).layer(inner_ok());
//         let request = Request::builder()
//             .method(Method::POST)
//             .uri("/x")
//             .body(())
//             .unwrap();

//         let response = svc.ready().await.unwrap().call(request).await.unwrap();
//         assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     }

//     #[tokio::test]
//     async fn post_with_matching_valid_cookie_and_header_is_allowed() {
//         let cfg = config();
//         let token = generate_signed_token(&cfg.secret);

//         let mut svc = CsrfLayer::new(cfg).layer(inner_ok());
//         let request = Request::builder()
//             .method(Method::POST)
//             .uri("/x")
//             .header(header::COOKIE, format!("csrf_token={token}"))
//             .header("x-csrf-token", token)
//             .body(())
//             .unwrap();

//         let response = svc.ready().await.unwrap().call(request).await.unwrap();
//         assert_eq!(response.status(), StatusCode::OK);
//     }

//     #[tokio::test]
//     async fn post_with_mismatched_cookie_and_header_is_rejected() {
//         let cfg = config();
//         let token = generate_signed_token(&cfg.secret);

//         let mut svc = CsrfLayer::new(cfg).layer(inner_ok());
//         let request = Request::builder()
//             .method(Method::POST)
//             .uri("/x")
//             .header(header::COOKIE, format!("csrf_token={token}"))
//             .header("x-csrf-token", "different-value")
//             .body(())
//             .unwrap();

//         let response = svc.ready().await.unwrap().call(request).await.unwrap();
//         assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     }

//     #[tokio::test]
//     async fn post_with_forged_cookie_matching_header_but_bad_signature_is_rejected() {
//         // 模拟攻击者猜测/伪造了一个值（cookie 和 header 一致，绕过了
//         // double-submit 这一步），但没有正确密钥、签名校验不通过。
//         let cfg = config();
//         let forged = "attacker-guessed-value.invalid-signature";

//         let mut svc = CsrfLayer::new(cfg).layer(inner_ok());
//         let request = Request::builder()
//             .method(Method::POST)
//             .uri("/x")
//             .header(header::COOKIE, format!("csrf_token={forged}"))
//             .header("x-csrf-token", forged)
//             .body(())
//             .unwrap();

//         let response = svc.ready().await.unwrap().call(request).await.unwrap();
//         assert_eq!(response.status(), StatusCode::FORBIDDEN);
//     }

//     // ---- Config ----

//     #[test]
//     fn short_secret_rejected() {
//         let cfg = CsrfConfig {
//             secret: "short".to_string(),
//             ..config()
//         };
//         assert!(matches!(
//             cfg.validate(),
//             Err(CsrfConfigError::SecretTooShort { .. })
//         ));
//     }
// }
