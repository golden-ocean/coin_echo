//! 统一管理 ORG 领域的 Casbin 权限标识符
pub mod org_policy {
    /// 组织架构管理权限
    pub mod organization {
        pub const CREATE: &str = "org:organization:create";
        pub const LIST: &str = "org:organization:list";
        pub const UPDATE: &str = "org:organization:update";
        pub const MOVE: &str = "org:organization:move";
        pub const DELETE: &str = "org:organization:delete";
    }

    /// 职位管理权限
    pub mod position {
        pub const CREATE: &str = "org:position:create";
        pub const LIST: &str = "org:position:list";
        pub const UPDATE: &str = "org:position:update";
        pub const DELETE: &str = "org:position:delete";
    }
}
