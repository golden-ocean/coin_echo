use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::id::{RoleId, UserId};
use uuid::Uuid;

pub struct UserAssignRolesCommand {
    pub user_id: Uuid,
    /// 该用户应拥有的完整角色 ID 集合（全量替换）
    pub role_ids: Vec<Uuid>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_user_assign_roles(
    uow_factory: &dyn UnitOfWorkFactory,
    cmd: UserAssignRolesCommand,
) -> Result<(), AppError> {
    let user_id_vo = UserId::from_uuid(cmd.user_id);
    let role_id_vos: Vec<RoleId> = cmd
        .role_ids
        .iter()
        .map(|id| RoleId::from_uuid(*id))
        .collect();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 1. 用户必须存在
                uow.user_repo()?
                    .find_by_id(&user_id_vo)
                    .await?
                    .ok_or(PortError::NotFound { entity: "user" })?;

                // 2. 逐一校验 role_id 是否真实存在，防止脏引用
                for rid in &role_id_vos {
                    uow.role_repo()?
                        .find_by_id(rid)
                        .await?
                        .ok_or(PortError::NotFound { entity: "role" })?;
                }

                // 3. 全量替换
                uow.user_role_repo()?
                    .replace_roles(&user_id_vo, &role_id_vos)
                    .await?;

                Ok(())
            })
        })
        .await?;

    // iam_user_role 同样没有审计字段，记录结构化日志作为操作痕迹补充
    tracing::info!(
        component = "handle_user_assign_roles",
        user_id = %cmd.user_id,
        role_count = cmd.role_ids.len(),
        operator_id = ?cmd.operator_id,
        "user roles replaced"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    // 建议覆盖的用例（同 role_assign_permissions 的测试思路）：
    // - test_assign_roles_success
    // - test_assign_roles_user_not_found
    // - test_assign_roles_role_not_found（且事务应整体回滚）
    // - test_assign_roles_empty_list
}
