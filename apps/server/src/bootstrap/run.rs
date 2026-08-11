//! 程序主流程：初始化基础设施，启动服务器，阻塞直到收到关闭信号。

use std::net::SocketAddr;
use std::sync::Arc;

use platform_config::ConfigMeta;
use platform_telemetry::TelemetryConfig;

use crate::bootstrap::{app, infra, shutdown};
use crate::config::ServerConfig;

pub async fn run() -> anyhow::Result<()> {
    platform_config::load_dotenv_if_present();

    let telemetry_cfg = TelemetryConfig::load()?;
    let _telemetry_guard = platform_telemetry::init(&telemetry_cfg)?;
    tracing::info!("日志已初始化");

    let state = Arc::new(infra::build_state().await?);
    tracing::info!("基础设施初始化完成");

    let server_cfg = ServerConfig::load()?;
    let app = app::build_app(Arc::clone(&state));

    let listener = tokio::net::TcpListener::bind(server_cfg.socket_addr()).await?;
    tracing::info!(addr = %server_cfg.socket_addr(), "服务器开始监听");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown::shutdown_signal(
        server_cfg.shutdown_grace_period(),
    ))
    .await?;

    tracing::info!("服务器已优雅关闭");
    Ok(())
}
