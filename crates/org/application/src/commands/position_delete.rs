use org_domain::id::PositionId;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{MembershipChecker, PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct PositionDeleteCommand {
    pub id: Uuid,
}

pub async fn handle_position_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    membership_checker: &dyn MembershipChecker,
    cmd: PositionDeleteCommand,
) -> Result<(), AppError> {
    let id = PositionId::from(cmd.id);

    let has_members = membership_checker.has_users_in_position(id.into()).await?;

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let position =
                    uow.position_repo()?.find_by_id(&id).await?.ok_or_else(|| {
                        AppError::from(PortError::NotFound { entity: "position" })
                    })?;

                position
                    .ensure_deletable(has_members)
                    .map_err(AppError::from)?;

                uow.position_repo()?.soft_delete(&position).await?;
                Ok(())
            })
        })
        .await
}
