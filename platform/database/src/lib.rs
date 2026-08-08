//! 数据库连接池与迁移。
//!
//! # 定位
//!
//! 只负责物理连接层：建立 read/write 连接池、运行 schema 迁移。不认识
//! 任何业务概念（CQRS、领域实体等）——那是 `iam-infra` 等业务 crate 的
//! 职责，它们持有 [`Pools`] 去读写数据。
//!
//! # read/write 分离
//!
//! [`config::DatabaseConfig::replica_url`] 是可选的。未配置时（当前场景：
//! 单一数据库），`Pools::read` 与 `Pools::write` 指向同一个连接池实例
//! （`PgPool` 内部是 `Arc`，`.clone()` 只克隆句柄，不建立新物理连接），
//! 代码路径与"真的有 replica"完全一致，业务代码不需要为"是否有 replica"
//! 写任何分支判断。以后接入真实只读副本，只需配置
//! `DATABASE_REPLICA_URL`，代码零改动。

mod pg;

pub use pg::{PgDatabaseConfig, PgPools};
