//! 优雅关闭信号监听。收到 SIGTERM/Ctrl-C 后停止接收新连接,
//! 等现有请求处理完再退出 —— 配合k8s滚动发布,避免请求被硬切断。

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!(target: "server::startup::graceful_shutdown", "received Ctrl+C, starting graceful shutdown"),
        _ = terminate => tracing::info!(target: "server::startup::graceful_shutdown", "received SIGTERM, starting graceful shutdown"),
    }
}
