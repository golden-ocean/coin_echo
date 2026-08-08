//! 服务启动入口。
//!
//! 仅一行调用；所有装配逻辑位于 `apps_server::app::run()`。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    apps_server::bootstrap::run().await
}

// // Initialize tracing for structured logging
//   tracing_subscriber::registry()
//       .with(tracing_subscriber::EnvFilter::try_from_default_env()
//           .unwrap_or_else(|_| "api_server=debug,tower_http=debug".into()))
//       .with(tracing_subscriber::fmt::layer().json())
//       .init();

//   // Load configuration
//   let config = Config::from_env();

//   // Create application state
//   let state = AppState::new(&config).await;

//   // Build router with all routes and middleware
//   let app = build_router(state);

//   // Start server with graceful shutdown
//   let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
//   tracing::info!("Starting server on {}", addr);

//   let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

//   axum::serve(listener, app)
//       .with_graceful_shutdown(shutdown_signal())
//       .await
//       .unwrap();

//   tracing::info!("Server shutdown complete");
// }
