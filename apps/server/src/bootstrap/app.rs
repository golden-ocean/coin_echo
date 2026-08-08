//! 基础设施初始化：database / cache / security。
use std::net::SocketAddr;

use crate::bootstrap::shutdown;

/// 程序入口：初始化全部基础设施，启动服务器，阻塞直到收到关闭信号。
pub async fn run() -> anyhow::Result<()> {
    // let listener = tokio::net::TcpListener::bind(server_config.bind_addr()).await?;
    // tracing::info!(addr = %server_config.bind_addr(), "服务器开始监听");

    // axum::serve(
    //     listener,
    //     app.into_make_service_with_connect_info::<SocketAddr>(),
    // )
    // .with_graceful_shutdown(shutdown::shutdown_signal())
    // .await?;

    // tracing::info!("服务器已优雅关闭");
    Ok(())
}
