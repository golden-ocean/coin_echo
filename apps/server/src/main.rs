//! 服务启动入口。
//!
//! 仅一行调用；所有装配逻辑位于 `apps_server::app::run()`。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    apps_server::bootstrap::run().await
}
