use std::str::FromStr;

use iam_domain::{
    error::DomainError,
    id::UserId,
    user::value_object::{Email, Phone},
};
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory},
};

pub struct UserUpdateCommand {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub organization_id: Option<Uuid>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_user_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: UserUpdateCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let user_id = UserId::from_uuid(cmd.id);
    let email_vo = Email::from_str(&cmd.email).map_err(DomainError::from)?;
    let phone_vo = Phone::from_str(&cmd.phone).map_err(DomainError::from)?;
    // let org_id_vo = cmd.organization_id.map(OrganizationId::from);
    let operator_id_vo = cmd.operator_id;

    let mut uow = uow_factory.begin().await?;

    // 1. 先查询用户是否存在，不存在直接返回 NotFound，避免后续无意义的唯一性查询
    let mut user = uow
        .user_repo()?
        .find_by_id(&user_id)
        .await?
        .ok_or(PortError::NotFound { entity: "user" })?;

    // 2. 只有当 username / email / phone 确实发生变化时，才需要查唯一性
    //    —— 性能优化点：避免值未变时的无意义查询，同时无需依赖仓库的 excluding_id 方法
    // if user.username() != &cmd.username && uow.user_repo().exists_by_username(&cmd.username).await?
    // {
    //     return Err(PortError::UniqueConflict {
    //         entity: "user",
    //         field: "username",
    //     }
    //     .into());
    // }

    if user.email() != &email_vo && uow.user_repo()?.exists_by_email(&email_vo).await? {
        return Err(PortError::UniqueConflict {
            entity: "user",
            field: "email",
        }
        .into());
    }

    if user.phone() != &phone_vo && uow.user_repo()?.exists_by_phone(&phone_vo).await? {
        return Err(PortError::UniqueConflict {
            entity: "user",
            field: "phone",
        }
        .into());
    }

    // 3. 在已查出的聚合根上应用变更（领域方法内部负责更新字段、递增 version、更新 audit_meta 等）
    user.update_info(cmd.name, email_vo, phone_vo, operator_id_vo, now)?;

    // 4. 落库（repo.update 内部通过 version 做乐观锁校验）
    uow.user_repo()?.update(&user).await?;
    uow.commit().await?;

    Ok(())
}
