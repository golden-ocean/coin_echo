//! 服务启动入口。

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    apps_server::run().await
}

// fn main() -> anyhow::Result<()> {
//     tokio::runtime::Builder::new_multi_thread()
//         .enable_all()
//         .thread_stack_size(8 * 1024 * 1024) // 8MB 栈大小生效
//         .build()?
//         .block_on(apps_server::run())
// }
