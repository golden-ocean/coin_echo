use std::{net::SocketAddr, time::Duration};

use platform_config::ConfigMeta;
use tokio::net::TcpListener;
use tracing::info;

use crate::{bootstrap::shutdown::shutdown_signal, state::AppState};

pub struct App {
    listener: TcpListener,
    // router: axum::Router,
    shutdown_grace_period: Duration,
}

impl App {
    /// 构建基础设施并完成路由组装
    pub async fn build() -> Result<Self, Box<dyn std::error::Error>> {
        // 加载配置
        let db_cfg = platform_database::PgDatabaseConfig::load()?;
        let server_cfg = platform_config::ServerConfig::load()?;

        // 初始化基础设施
        let pools = platform_database::PgPools::connect(&db_cfg).await?;
        info!("Database pools connected and verified.");

        // 构造 AppState 与 Router
        let state = AppState::new(pools);
        // let router = routes::build_router(state);

        // 绑定 TCP 端口
        let addr: SocketAddr = server_cfg.socket_addr();
        let listener = TcpListener::bind(addr).await?;
        info!("Server bound to {}", addr);

        Ok(Self {
            listener,
            // router,
            shutdown_grace_period: server_cfg.shutdown_grace_period(),
        })
    }

    /// 启动 HTTP 服务并阻塞直至收到关闭信号
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        axum::serve(self.listener, self.router)
            .with_graceful_shutdown(shutdown_signal(self.shutdown_grace_period))
            .await?;

        info!("Server shut down gracefully.");
        Ok(())
    }
}
