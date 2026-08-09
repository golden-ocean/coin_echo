//! 日志订阅器的组装与安装。

use std::io;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer, Registry};

use crate::config::{LogFormat, TelemetryConfig};
use crate::error::TelemetryError;

/// 持有初始化过程中产生的资源句柄（非阻塞写入器的后台线程）。
///
/// 必须持有到进程退出前：`Drop` 时会 flush 缓冲区中尚未写出的日志。
#[must_use = "丢弃返回值会导致非阻塞日志写入器提前关闭，缓冲中的日志将丢失"]
pub struct TelemetryGuard {
    _appender_guard: tracing_appender::non_blocking::WorkerGuard,
}

/// 初始化全局日志订阅器。整个进程生命周期内只应调用一次。
pub fn init(config: &TelemetryConfig) -> Result<TelemetryGuard, TelemetryError> {
    let filter = EnvFilter::try_new(config.filter_directive())
        .map_err(|e| TelemetryError::InvalidFilter(e.to_string()))?;

    let (non_blocking_writer, appender_guard) = tracing_appender::non_blocking(io::stdout());
    let fmt_layer = build_fmt_layer(config, non_blocking_writer);

    tracing_subscriber::registry()
        .with(fmt_layer) // 先叠加在裸 Registry 上，Layer<Registry> 类型匹配
        .with(filter) // EnvFilter 对任意 S 都实现 Layer<S>，放最后不受影响
        .try_init()
        .map_err(|_| TelemetryError::AlreadyInitialized)?;

    warn_if_otel_requested_but_unimplemented(config);

    Ok(TelemetryGuard {
        _appender_guard: appender_guard,
    })
}

/// OTel 尚未实现的桩函数。
///
/// # 为什么现在就写这个函数，而不是等真正实现时再加
///
/// 让"配置里打开了 otel_enabled，但功能还没做"这件事，从"配置被静默
/// 忽略"变成"启动时有一条清晰的 warn 日志"，避免有人以为设了
/// `TELEMETRY_OTEL_ENABLED=true` 就真的在导出追踪数据。等真正实现 OTel
/// 时，这个函数会被替换成真实的 `opentelemetry-otlp` 初始化逻辑，函数
/// 签名（接收 `&TelemetryConfig`，可能返回一个额外的 guard）预计不需要
/// 大改，调用点（[`init`] 内部这一行）也不需要变。
fn warn_if_otel_requested_but_unimplemented(config: &TelemetryConfig) {
    if config.otel_enabled {
        tracing::warn!(
            otlp_endpoint = ?config.otlp_endpoint,
            "TELEMETRY_OTEL_ENABLED=true，但本项目尚未实现 OTel 导出，该配置暂不生效"
        );
    }
}

/// 按配置组装格式化层。
fn build_fmt_layer(
    config: &TelemetryConfig,
    writer: tracing_appender::non_blocking::NonBlocking,
) -> Box<dyn Layer<Registry> + Send + Sync> {
    match config.format {
        LogFormat::Json => Box::new(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(writer)
                .with_target(true)
                .with_current_span(true),
        ),
        LogFormat::Pretty => Box::new(
            tracing_subscriber::fmt::layer()
                .pretty()
                .with_writer(writer)
                .with_ansi(config.ansi)
                .with_target(true),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 不测试 `init()` 对全局订阅器的安装效果本身——`tracing` 的全局
    // 订阅器进程内只能设置一次，多个测试并行调用会互相冲突。这里只测试
    // 不依赖全局状态的部分。

    #[test]
    fn valid_level_produces_parsable_filter() {
        let cfg = TelemetryConfig::default();
        assert!(EnvFilter::try_new(cfg.filter_directive()).is_ok());
    }

    #[test]
    fn invalid_directive_syntax_is_rejected() {
        let cfg = TelemetryConfig {
            extra_directives: Some("this is not a valid directive===".to_string()),
            ..TelemetryConfig::default()
        };
        assert!(EnvFilter::try_new(cfg.filter_directive()).is_err());
    }

    #[test]
    fn build_fmt_layer_does_not_panic_for_json_format() {
        let cfg = TelemetryConfig {
            format: LogFormat::Json,
            ..TelemetryConfig::default()
        };
        let (writer, _guard) = tracing_appender::non_blocking(io::sink());
        let _layer = build_fmt_layer(&cfg, writer);
    }

    #[test]
    fn build_fmt_layer_does_not_panic_for_pretty_format() {
        let cfg = TelemetryConfig {
            format: LogFormat::Pretty,
            ..TelemetryConfig::default()
        };
        let (writer, _guard) = tracing_appender::non_blocking(io::sink());
        let _layer = build_fmt_layer(&cfg, writer);
    }

    #[test]
    fn init_returns_invalid_filter_error_without_touching_global_state() {
        let cfg = TelemetryConfig {
            extra_directives: Some("===invalid===".to_string()),
            ..TelemetryConfig::default()
        };
        let result = init(&cfg);
        assert!(matches!(result, Err(TelemetryError::InvalidFilter(_))));
    }

    #[test]
    fn otel_stub_does_not_panic_when_enabled() {
        // warn_if_otel_requested_but_unimplemented 是纯粹的日志副作用，
        // 这里只验证调用路径安全，不断言日志内容（无 subscriber 时
        // tracing 宏调用本身是安全的空操作）。
        let cfg = TelemetryConfig {
            otel_enabled: true,
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            ..TelemetryConfig::default()
        };
        warn_if_otel_requested_but_unimplemented(&cfg);
    }
}
