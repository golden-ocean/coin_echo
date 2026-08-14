use std::str::FromStr;

use iam_domain::{
    error::DomainError,
    id::RoleId,
    role::value_object::{RoleCode, RoleName},
};
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory},
};

pub struct RoleUpdateCommand {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_role_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: RoleUpdateCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let role_id = RoleId::from_uuid(cmd.id);
    let name_vo = RoleName::from_str(&cmd.name).map_err(DomainError::from)?;
    let code_vo = RoleCode::from_str(&cmd.code).map_err(DomainError::from)?;
    let operator_id_vo = cmd.operator_id;

    let mut uow = uow_factory.begin().await?;

    // 1. 先查询角色是否存在，不存在直接返回 NotFound，避免后续无意义的唯一性查询
    let mut role = uow
        .role_repo()?
        .find_by_id(&role_id)
        .await?
        .ok_or(PortError::NotFound { entity: "role" })?;

    // 2. 只有当 code / name 确实发生变化时，才需要查唯一性
    //    —— 性能优化点：避免值未变时的无意义查询，同时排除“查到自己”导致的误判冲突
    if role.code() != &code_vo && uow.role_repo()?.exists_by_code(&code_vo).await? {
        return Err(PortError::UniqueConflict {
            entity: "role",
            field: "code",
        }
        .into());
    }
    if role.name() != &name_vo && uow.role_repo()?.exists_by_name(&name_vo).await? {
        return Err(PortError::UniqueConflict {
            entity: "role",
            field: "name",
        }
        .into());
    }

    // 3. 在已查出的聚合根上应用变更（领域方法内部负责递增 version、更新 updated_at 等审计字段）
    role.update_info(name_vo, code_vo, cmd.remark, cmd.sort, operator_id_vo, now)?;

    // 4. 落库（repo.update 内部通过 version 做乐观锁校验）
    uow.role_repo()?.update(&role).await?;
    uow.commit().await?;
    Ok(())
}
