use crate::ports::error::PortError;
use iam_domain::id::{PermissionId, RoleId};

/// 角色-权限 中间表仓储端口
///
/// 只提供全量替换，语义和交互场景是"角色权限勾选树"。
#[async_trait::async_trait]
pub trait RolePermissionRepository: Send + Sync {
    /// 全量替换某角色的权限集合
    async fn replace_permissions(
        &mut self,
        role_id: &RoleId,
        permission_ids: &[PermissionId],
    ) -> Result<(), PortError>;

    /// 查询某角色当前拥有的所有权限 ID（用于前端权限树回显）
    async fn list_permission_ids_by_role(
        &mut self,
        role_id: &RoleId,
    ) -> Result<Vec<PermissionId>, PortError>;

    /// 查询某权限当前被哪些角色持有（用于"删除权限前检查是否被引用"这类前置校验，
    /// 以及 Casbin 策略同步时反查"这个权限影响哪些角色"）
    async fn list_role_ids_by_permission(
        &mut self,
        permission_id: &PermissionId,
    ) -> Result<Vec<RoleId>, PortError>;
}
