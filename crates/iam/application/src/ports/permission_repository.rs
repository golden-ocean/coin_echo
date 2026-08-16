use iam_domain::{
    id::PermissionId,
    permission::{Permission, value_object::PermissionCode, value_object::PermissionName},
};

use crate::ports::error::PortError;

#[async_trait::async_trait]
pub trait PermissionRepository: Send + Sync {
    async fn insert(&mut self, permission: &Permission) -> Result<(), PortError>;
    async fn update(&mut self, permission: &Permission) -> Result<(), PortError>;
    async fn soft_delete(&mut self, permission: &Permission) -> Result<(), PortError>;

    async fn find_by_id(
        &mut self,
        permission_id: &PermissionId,
    ) -> Result<Option<Permission>, PortError>;
    async fn find_by_code(
        &mut self,
        code: &PermissionCode,
    ) -> Result<Option<Permission>, PortError>;

    async fn exists_by_code(&mut self, code: &PermissionCode) -> Result<bool, PortError>;
    async fn exists_by_name(&mut self, name: &PermissionName) -> Result<bool, PortError>;

    /// 查询某个父级下的直接子权限（parent_id = None 表示查所有根节点）
    async fn find_by_parent_id(
        &mut self,
        parent_id: Option<PermissionId>,
    ) -> Result<Vec<Permission>, PortError>;

    /// 判断某个权限是否存在子权限（用于删除/禁用前的前置校验，
    /// 防止孤儿节点或级联影响未被感知）
    async fn has_children(&mut self, id: &PermissionId) -> Result<bool, PortError>;

    /// 判断 `ancestor_id` 是否是 `descendant_id` 的祖先节点（含多级）。
    /// 用于 `Permission::change_parent` 调用前的多级循环引用校验——
    /// 聚合根内部只能拦截"设为自己"这种直接自环，多级 A→B→C→A 的检测
    /// 依赖仓储层遍历树结构，因此该校验放在这里而不是聚合根方法内。
    async fn is_ancestor(
        &mut self,
        ancestor_id: &PermissionId,
        descendant_id: &PermissionId,
    ) -> Result<bool, PortError>;
}
