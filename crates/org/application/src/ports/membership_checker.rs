use uuid::Uuid;

use crate::ports::PortError;

/// 跨模块查询端口：判断组织/职位下是否还有用户挂载
/// 具体实现在 infra 层，可以是直接查 iam_user 表（单体应用），
/// 也可以是未来拆分服务后的 RPC 调用——application 层不关心实现细节
#[async_trait::async_trait]
pub trait MembershipChecker: Send + Sync {
    async fn has_users_in_organization(&self, organization_id: Uuid) -> Result<bool, PortError>;
    async fn has_users_in_position(&self, position_id: Uuid) -> Result<bool, PortError>;
}
