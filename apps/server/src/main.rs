//! 服务启动入口。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    apps_server::run().await
}
