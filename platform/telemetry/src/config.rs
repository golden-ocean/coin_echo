//! 遥测配置。
//!
//! 对应环境变量前缀 `TELEMETRY_`。

use std::str::FromStr;

use platform_config::ConfigMeta;
use serde::{Deserialize, Deserializer};

/// 日志输出格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// 结构化 JSON，一行一条，便于日志采集系统（Loki/ELK 等）解析。
    /// 生产环境使用。
    Json,
    /// 彩色、带缩进的人类可读格式。本地开发使用。
    Pretty,
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "pretty" => Ok(Self::Pretty),
            other => Err(format!("未知的日志格式：{other}，应为 json 或 pretty")),
        }
    }
}

/// 手写而非 `#[derive(Deserialize)]`：`envy` 把环境变量值当作裸字符串
/// 交给 serde 解析，对 Rust 原生的 C 风格枚举支持并不可靠；改为
/// "先取字符串、再用 [`FromStr`] 解析" 是明确且经过验证有效的路径。
impl<'de> Deserialize<'de> for LogFormat {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// 遥测（日志 + 预留的分布式追踪）配置。
#[derive(Debug, Clone, Deserialize)]
pub struct TelemetryConfig {
    /// 默认日志级别：`trace`/`debug`/`info`/`warn`/`error`。
    #[serde(default = "TelemetryConfig::default_level")]
    pub level: String,

    /// 输出格式。
    #[serde(default = "TelemetryConfig::default_format")]
    pub format: LogFormat,

    /// 是否输出 ANSI 颜色码。仅 [`LogFormat::Pretty`] 下生效。
    #[serde(default = "TelemetryConfig::default_ansi")]
    pub ansi: bool,

    /// 追加的按模块过滤指令，逗号分隔，如 `sqlx=warn,tower_http=info`。
    #[serde(default)]
    pub extra_directives: Option<String>,

    /// 服务名称，用于日志字段与（未来）OTel 资源属性（`service.name`）。
    #[serde(default = "TelemetryConfig::default_service_name")]
    pub service_name: String,

    /// 是否启用 OTel 追踪导出。
    ///
    /// **当前尚未实现**——本 crate 目前不依赖任何 `opentelemetry*` 包，
    /// 该字段只是预留数据位。[`crate::init::init`] 读到 `true` 时仅记录
    /// 一条 warn 日志说明该功能尚未启用，不会报错、不会导致启动失败，
    /// 以保证配置向前兼容：现在把这个开关打开，等真正实现后自动生效，
    /// 不需要因为提前打开配置而报错。
    #[serde(default)]
    pub otel_enabled: bool,

    /// OTLP collector 地址（如 `http://localhost:4317`）。仅在
    /// `otel_enabled = true` 且未来完成实现后生效。
    #[serde(default)]
    pub otlp_endpoint: Option<String>,

    /// 采样比例（0.0～1.0）。仅在 OTel 启用后生效。
    #[serde(default = "TelemetryConfig::default_sample_ratio")]
    pub sample_ratio: f64,
}

/// 配置语义层面的非法状态。
#[derive(Debug, thiserror::Error)]
pub enum TelemetryConfigError {
    #[error("sample_ratio 必须在 0.0 到 1.0 之间，当前为 {0}")]
    SampleRatioOutOfRange(f64),

    #[error("otel_enabled 为 true 时，otlp_endpoint 不能为空")]
    MissingOtlpEndpoint,
}

impl TelemetryConfig {
    fn default_level() -> String {
        "info".to_string()
    }

    const fn default_format() -> LogFormat {
        LogFormat::Pretty
    }

    const fn default_ansi() -> bool {
        true
    }

    fn default_service_name() -> String {
        "app".to_string()
    }

    const fn default_sample_ratio() -> f64 {
        1.0
    }

    /// 组装完整的过滤表达式字符串，供 [`tracing_subscriber::EnvFilter`] 解析。
    #[must_use]
    pub fn filter_directive(&self) -> String {
        match &self.extra_directives {
            Some(extra) if !extra.trim().is_empty() => format!("{},{extra}", self.level),
            _ => self.level.clone(),
        }
    }
}

impl ConfigMeta for TelemetryConfig {
    type Error = TelemetryConfigError;

    fn prefix() -> &'static str {
        "TELEMETRY_"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        if !(0.0..=1.0).contains(&self.sample_ratio) {
            return Err(TelemetryConfigError::SampleRatioOutOfRange(
                self.sample_ratio,
            ));
        }
        if self.otel_enabled
            && self
                .otlp_endpoint
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(TelemetryConfigError::MissingOtlpEndpoint);
        }
        Ok(())
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            level: Self::default_level(),
            format: Self::default_format(),
            ansi: Self::default_ansi(),
            extra_directives: None,
            service_name: Self::default_service_name(),
            otel_enabled: false,
            otlp_endpoint: None,
            sample_ratio: Self::default_sample_ratio(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- LogFormat::from_str ----

    #[test]
    fn parses_json_case_insensitively() {
        assert_eq!("json".parse::<LogFormat>(), Ok(LogFormat::Json));
        assert_eq!("JSON".parse::<LogFormat>(), Ok(LogFormat::Json));
    }

    #[test]
    fn parses_pretty_case_insensitively() {
        assert_eq!("pretty".parse::<LogFormat>(), Ok(LogFormat::Pretty));
    }

    #[test]
    fn rejects_unknown_format_string() {
        assert!("xml".parse::<LogFormat>().is_err());
    }

    // ---- TelemetryConfig 默认值与过滤表达式 ----

    #[test]
    fn defaults_are_info_pretty_ansi_enabled_otel_disabled() {
        let cfg = TelemetryConfig::default();
        assert_eq!(cfg.level, "info");
        assert_eq!(cfg.format, LogFormat::Pretty);
        assert!(cfg.ansi);
        assert!(cfg.extra_directives.is_none());
        assert!(!cfg.otel_enabled);
        assert!(cfg.otlp_endpoint.is_none());
        assert_eq!(cfg.sample_ratio, 1.0);
    }

    #[test]
    fn filter_directive_is_bare_level_without_extra_directives() {
        let cfg = TelemetryConfig::default();
        assert_eq!(cfg.filter_directive(), "info");
    }

    #[test]
    fn filter_directive_appends_extra_directives_after_level() {
        let cfg = TelemetryConfig {
            extra_directives: Some("sqlx=warn,tower_http=debug".to_string()),
            ..TelemetryConfig::default()
        };
        assert_eq!(cfg.filter_directive(), "info,sqlx=warn,tower_http=debug");
    }

    #[test]
    fn filter_directive_ignores_blank_extra_directives() {
        let cfg = TelemetryConfig {
            extra_directives: Some("   ".to_string()),
            ..TelemetryConfig::default()
        };
        assert_eq!(cfg.filter_directive(), "info");
    }

    // ---- validate() ----

    #[test]
    fn default_config_passes_validation() {
        assert!(TelemetryConfig::default().validate().is_ok());
    }

    #[test]
    fn sample_ratio_above_one_rejected() {
        let cfg = TelemetryConfig {
            sample_ratio: 1.5,
            ..TelemetryConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(TelemetryConfigError::SampleRatioOutOfRange(_))
        ));
    }

    #[test]
    fn negative_sample_ratio_rejected() {
        let cfg = TelemetryConfig {
            sample_ratio: -0.1,
            ..TelemetryConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(TelemetryConfigError::SampleRatioOutOfRange(_))
        ));
    }

    #[test]
    fn otel_enabled_without_endpoint_rejected() {
        let cfg = TelemetryConfig {
            otel_enabled: true,
            otlp_endpoint: None,
            ..TelemetryConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(TelemetryConfigError::MissingOtlpEndpoint)
        ));
    }

    #[test]
    fn otel_enabled_with_endpoint_passes() {
        let cfg = TelemetryConfig {
            otel_enabled: true,
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            ..TelemetryConfig::default()
        };
        assert!(cfg.validate().is_ok());
    }

    // ---- ConfigMeta::load_from ----

    #[test]
    fn load_from_applies_defaults_when_no_vars_present() {
        let cfg = TelemetryConfig::load_from(Vec::<(String, String)>::new()).unwrap();
        assert_eq!(cfg.level, "info");
        assert!(!cfg.otel_enabled);
    }

    #[test]
    fn load_from_rejects_invalid_sample_ratio() {
        let vars = vec![("TELEMETRY_SAMPLE_RATIO".to_string(), "2.0".to_string())];
        let result = TelemetryConfig::load_from(vars);
        assert!(matches!(
            result,
            Err(platform_config::ConfigError::Validation { .. })
        ));
    }

    #[test]
    fn prefix_is_telemetry() {
        assert_eq!(TelemetryConfig::prefix(), "TELEMETRY_");
    }
}
