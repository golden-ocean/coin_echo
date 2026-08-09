//! Redis 连接池。
//!
//! # 定位
//!
//! 只负责物理连接层：建立连接池、健康检查。不封装任何业务语义的缓存操作
//! （如"缓存用户资料"这类带 key 命名规则、序列化格式、TTL 策略的封装）——
//! 那是各业务领域 `xxx-infra` 的职责，它们持有 [`RedisPool`] 去读写。
//!
//! 之所以不在这里提供 `get`/`set` 之类的通用方法：Redis 命令集合很大
//! （string/hash/list/set/zset/pub-sub...），业务用到的往往只是其中一小
//! 部分，且几乎总是搭配序列化格式（JSON/bincode）与 key 命名规则一起使用
//! ——这些都是业务决策，包在这里只会变成一层不贴合任何具体用法的转发。

mod config;
mod error;
mod pool;

pub use config::RedisConfig;
pub use pool::RedisPool;
