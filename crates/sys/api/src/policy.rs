//! 统一管理与 Casbin 权限中间件和前端鉴权挂钩的 Permission Code

pub mod sys_policy {
    /// 字典管理权限
    pub mod dictionary {
        pub const CREATE: &str = "sys:dictionary:create";
        pub const LIST: &str = "sys:dictionary:list";
        pub const UPDATE: &str = "sys:dictionary:update";
        pub const DELETE: &str = "sys:dictionary:delete";
    }

    /// 字典项管理权限
    pub mod dictionary_item {
        pub const CREATE: &str = "sys:dictionary_item:create";
        pub const PAGE: &str = "sys:dictionary_item:page";
        pub const UPDATE: &str = "sys:dictionary_item:update";
        pub const DELETE: &str = "sys:dictionary_item:delete";
    }
}
