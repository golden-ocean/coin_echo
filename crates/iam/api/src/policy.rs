//! 统一管理 IAM 领域的 Casbin 权限标识符

pub mod iam_policy {
    /// 用户管理权限
    pub mod user {
        pub const CREATE: &str = "iam:user:create";
        pub const PAGE: &str = "iam:user:page";
        pub const UPDATE: &str = "iam:user:update";
        pub const DELETE: &str = "iam:user:delete";
    }

    /// 角色管理权限
    pub mod role {
        pub const CREATE: &str = "iam:role:create";
        pub const PAGE: &str = "iam:role:page";
        pub const UPDATE: &str = "iam:role:update";
        pub const DELETE: &str = "iam:role:delete";
    }

    /// 权限节点管理权限
    pub mod permission {
        pub const CREATE: &str = "iam:permission:create";
        pub const LIST: &str = "iam:permission:list";
        pub const UPDATE: &str = "iam:permission:update";
        pub const DELETE: &str = "iam:permission:delete";
    }
}
