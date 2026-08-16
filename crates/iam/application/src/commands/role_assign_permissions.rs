use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::id::{PermissionId, RoleId};
use uuid::Uuid;

pub struct RoleAssignPermissionsCommand {
    pub role_id: Uuid,
    /// 该角色应拥有的完整权限 ID 集合（全量替换，不是增量 assign）
    pub permission_ids: Vec<Uuid>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_role_assign_permissions(
    uow_factory: &dyn UnitOfWorkFactory,
    cmd: RoleAssignPermissionsCommand,
) -> Result<(), AppError> {
    let role_id_vo = RoleId::from_uuid(cmd.role_id);
    let permission_id_vos: Vec<PermissionId> = cmd
        .permission_ids
        .iter()
        .map(|id| PermissionId::from_uuid(*id))
        .collect();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 1. 角色必须存在
                uow.role_repo()?
                    .find_by_id(&role_id_vo)
                    .await?
                    .ok_or(PortError::NotFound { entity: "role" })?;

                // 2. 逐一校验 permission_id 是否真实存在，防止脏引用
                //    （权限量级通常在几十到几百，逐条查询在事务内可接受；
                //    未来量级显著增大可给 PermissionRepository 加批量存在性校验方法）
                for pid in &permission_id_vos {
                    uow.permission_repo()?
                        .find_by_id(pid)
                        .await?
                        .ok_or(PortError::NotFound {
                            entity: "permission",
                        })?;
                }

                // 3. 全量替换（差集比对逻辑封装在 Repository 内部，只对变化部分做 DML）
                uow.role_permission_repo()?
                    .replace_permissions(&role_id_vo, &permission_id_vos)
                    .await?;

                Ok(())
            })
        })
        .await?;

    // iam_role_permission 表没有 created_by/updated_by 审计字段（纯关系表），
    // 用结构化日志记录一条操作痕迹作为审计补充，避免这类敏感操作完全无迹可查
    tracing::info!(
        component = "handle_role_assign_permissions",
        role_id = %cmd.role_id,
        permission_count = cmd.permission_ids.len(),
        operator_id = ?cmd.operator_id,
        "role permissions replaced"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    // 建议覆盖的用例（需要配合 mock UnitOfWork/Repository 或集成测试数据库）：
    // - test_assign_permissions_success：角色和所有权限都存在，替换成功
    // - test_assign_permissions_role_not_found：role_id 不存在，返回 PortError::NotFound { entity: "role" }
    // - test_assign_permissions_permission_not_found：permission_ids 中某个 ID 不存在，
    //   返回 PortError::NotFound { entity: "permission" }，且事务应整体回滚
    //   （即之前已校验通过的权限也不应该被写入）
    // - test_assign_permissions_empty_list：permission_ids 为空数组，应该清空该角色的所有权限
}

