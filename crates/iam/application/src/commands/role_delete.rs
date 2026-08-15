use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::id::RoleId;
use platform_kernel::time::Clock;
use uuid::Uuid;

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

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let mut role = uow
                    .role_repo()?
                    .find_by_id(&role_id)
                    .await?
                    .ok_or(PortError::NotFound { entity: "role" })?;

                role.delete(operator_id_vo, now)?;
                uow.role_repo()?.soft_delete(&role).await?;
                Ok(())
            })
        })
        .await
}
