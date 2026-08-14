use iam_domain::id::RoleId;
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory},
};

pub struct RoleDeleteCommand {
    pub id: Uuid,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_role_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: RoleDeleteCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let role_id = RoleId::from_uuid(cmd.id);
    let operator_id_vo = cmd.operator_id;

    let mut uow = uow_factory.begin().await?;

    let mut role = uow
        .role_repo()?
        .find_by_id(&role_id)
        .await?
        .ok_or(PortError::NotFound { entity: "role" })?;

    role.delete(operator_id_vo, now)?;

    uow.role_repo()?.soft_delete(&role).await?;
    uow.commit().await?;
    Ok(())
}
