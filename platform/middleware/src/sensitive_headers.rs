//! 敏感请求/响应头脱敏。
//!
//! # 作用
//!
//! 把 `Authorization`（携带 JWT/Basic 认证凭证）、`Cookie`、
//! `Set-Cookie` 这几个头标记为"敏感"。标记后，[`super::trace`] 里
//! `TraceLayer` 生成的访问日志在打印请求/响应头时，会自动把这些头的值
//! 替换成 `Sensitive`，不会把凭证明文写进日志——这是防止认证令牌随着
//! 日志采集系统扩散出去的最后一道防线。
//!
//! # 使用的 tower-http 组件
//!
//! [`SetSensitiveRequestHeadersLayer`]/[`SetSensitiveResponseHeadersLayer`]：
//! 纯粹的标记层，不修改请求/响应本身的内容，只是给指定的头打上一个
//! "调用 Debug 格式化时应隐藏"的标记，实际生效位置在 `TraceLayer`
//! 内部读取请求/响应头做日志格式化的那一步。
//!
//! # 为什么要分请求头和响应头两层
//!
//! `Authorization`/`Cookie` 通常只出现在请求里，`Set-Cookie` 通常只出现
//! 在响应里；tower-http 把"标记请求头敏感"和"标记响应头敏感"设计成两个
//! 独立的层，因为它们分别作用于请求处理的不同阶段（进入时 vs 返回时）。
//!
//! # 应用位置（在 `apply.rs` 中）
//!
//! 必须放在 [`super::trace`] 之外（比 trace 层更早执行，即在
//! `Router::layer()` 的调用顺序上晚于 trace 层——回顾模块文档："后调用
//! 的 `.layer()` 是最外层、最先执行"）。若顺序反了，`TraceLayer` 记录
//! 日志时看到的还是未打标记的原始头，脱敏不会生效。

use http::header;
use tower_http::sensitive_headers::{
    SetSensitiveRequestHeadersLayer, SetSensitiveResponseHeadersLayer,
};

/// 需要脱敏的头列表。集中定义在一处，新增需要脱敏的头（如未来引入
/// API Key 认证用的自定义头）时只改这一个数组。
const SENSITIVE_HEADERS: [http::HeaderName; 3] =
    [header::AUTHORIZATION, header::COOKIE, header::SET_COOKIE];

/// 请求头脱敏层。
pub fn request_layer() -> SetSensitiveRequestHeadersLayer {
    SetSensitiveRequestHeadersLayer::new(SENSITIVE_HEADERS)
}

/// 响应头脱敏层。
pub fn response_layer() -> SetSensitiveResponseHeadersLayer {
    SetSensitiveResponseHeadersLayer::new(SENSITIVE_HEADERS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_header_list_covers_auth_and_cookie() {
        // 这里不做端到端断言（"打了标记的头在 Debug 输出里被隐藏"这个
        // 效果本质是 TraceLayer 内部读取标记位实现的，属于集成行为，
        // 单靠这两个 Layer 本身无法独立验证）。只做一个静态检查：
        // 确保列表里包含了三个关键头，防止后续有人误删。
        assert!(SENSITIVE_HEADERS.contains(&header::AUTHORIZATION));
        assert!(SENSITIVE_HEADERS.contains(&header::COOKIE));
        assert!(SENSITIVE_HEADERS.contains(&header::SET_COOKIE));
    }
}
