//! 成功响应的统一信封（Envelope）。
//!
//! # `trace_id` 由调用方传入
//!
//! 本模块不关心 trace_id 具体怎么取（不依赖任何 tracing/otel 实现），由最外层
//! 中间件统一获取后传入 `Res::ok`。取值方式将在 `platform-telemetry` 里给出。

use serde::Serialize;

/// 成功响应。
#[derive(Debug, Serialize)]
pub struct Res<T: Serialize> {
    /// 业务数据。`None` 用于"操作成功但无返回内容"的场景（如 DELETE）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 链路追踪 ID，由中间件统一回填，构造时原样传入。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// 响应生成时间（Unix 毫秒时间戳）。
    pub timestamp: i64,
}

impl<T: Serialize> Res<T> {
    /// 构造带数据的成功响应。
    #[must_use]
    pub fn ok(data: T, trace_id: Option<String>) -> Self {
        Self {
            data: Some(data),
            trace_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

impl Res<()> {
    /// 构造无数据的成功响应（如 DELETE、状态变更类接口）。
    #[must_use]
    pub fn empty(trace_id: Option<String>) -> Self {
        Self {
            data: None,
            trace_id,
            timestamp: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ok_wraps_data_and_omits_none_trace_id() {
        let res = Res::ok(42, None);
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"data\":42"));
        assert!(!json.contains("trace_id"));
    }

    #[test]
    fn ok_includes_trace_id_when_present() {
        let res = Res::ok("hello", Some("trace-abc".to_string()));
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"trace_id\":\"trace-abc\""));
    }

    #[test]
    fn empty_has_no_data_field_in_json() {
        let res = Res::<()>::empty(None);
        let json = serde_json::to_string(&res).unwrap();
        assert!(!json.contains("\"data\""));
        assert!(json.contains("\"timestamp\""));
    }
}
