//! 数据库相关错误。

use platform_kernel::error::{ErrorKind, ErrorMeta};

/// 数据库连接与迁移过程中的错误。
///
/// 不包含"某条查询失败"这类运行期错误——那些错误发生在具体的
/// repository 实现里（如 `iam-infra`），由该 crate 自己的错误类型
/// 包裹 `sqlx::Error` 并结合业务语义分类（比如唯一约束冲突该映射成
/// `ErrorKind::Conflict` 还是别的，取决于是哪个字段冲突，这个判断
/// `platform-database` 做不出来，必须留给认识 schema 语义的业务层）。
/// 这里只覆盖"连接池建不起来""迁移跑不通"这类启动阶段的故障。
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    /// 建立连接池失败：地址不可达、认证失败、连接数配置非法等。
    #[error("建立数据库连接池失败：{0}")]
    ConnectFailed(#[source] sqlx::Error),

    /// 运行 schema 迁移失败。
    #[error("运行数据库迁移失败：{0}")]
    MigrationFailed(#[source] sqlx::migrate::MigrateError),
}

impl ErrorMeta for DatabaseError {
    fn kind(&self) -> ErrorKind {
        // 两者都发生在启动阶段，且原因几乎总是环境/配置问题（连不上库、
        // 迁移脚本冲突），不是某次用户请求触发的，统一归为 Unavailable：
        // 依赖不可用，符合语义，且默认可重试（进程重启后可能就好了）。
        ErrorKind::Unavailable
    }

    fn code(&self) -> &'static str {
        match self {
            Self::ConnectFailed(_) => "database.connect_failed",
            Self::MigrationFailed(_) => "database.migration_failed",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn startup_failures_are_unavailable_and_retryable() {
        // 用一个真实能构造的 sqlx::Error 变体验证分类，而非 mock——
        // sqlx::Error::Configuration 是公开可构造的最简单变体。
        let err = DatabaseError::ConnectFailed(sqlx::Error::Configuration("bad config".into()));
        assert_eq!(err.kind(), ErrorKind::Unavailable);
        assert!(err.retryable());
    }

    #[test]
    fn codes_are_stable_and_distinct() {
        let connect_err = DatabaseError::ConnectFailed(sqlx::Error::Configuration("x".into()));
        assert_eq!(connect_err.code(), "database.connect_failed");
    }
}
