use crate::ports::error::PortError;
use iam_domain::id::{RoleId, UserId};

/// 用户-角色 中间表仓储端口
///
/// 设计说明：只提供 `replace_roles` 全量替换，不提供逐条 assign/revoke。
/// 前端"给用户分配角色"的交互几乎总是一个多选框，提交的是"这个用户现在应该
/// 拥有的完整角色列表"，后端职责是 diff（该删的删、该加的加），而不是维护一堆
/// 零散的增删接口——这样也天然规避了"重复分配"这类边界情况。
#[async_trait::async_trait]
pub trait UserRoleRepository: Send + Sync {
    /// 全量替换某用户的角色集合
    async fn replace_roles(
        &mut self,
        user_id: &UserId,
        role_ids: &[RoleId],
    ) -> Result<(), PortError>;

    /// 查询某用户当前拥有的所有角色 ID（用于前端回显已选中的角色）
    async fn list_role_ids_by_user(&mut self, user_id: &UserId) -> Result<Vec<RoleId>, PortError>;

    /// 查询某角色当前被哪些用户持有（用于"删除角色前检查是否有人在用"这类前置校验）
    async fn list_user_ids_by_role(&mut self, role_id: &RoleId) -> Result<Vec<UserId>, PortError>;
}
