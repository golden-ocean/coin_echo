use org_domain::{
    error::DomainError,
    id::PositionId,
    position::value_object::{PositionCode, PositionName},
};
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct PositionUpdateCommand {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub remark: Option<String>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_position_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: PositionUpdateCommand,
) -> Result<(), AppError> {
    let id = PositionId::from(cmd.id);
    let name_vo = PositionName::try_from(cmd.name).map_err(DomainError::from)?;
    let code_vo = PositionCode::try_from(cmd.code).map_err(DomainError::from)?;
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let mut position =
                    uow.position_repo()?.find_by_id(&id).await?.ok_or_else(|| {
                        AppError::from(PortError::NotFound { entity: "position" })
                    })?;

                if position.name() != &name_vo
                    && uow.position_repo()?.exists_by_name(&name_vo).await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "position",
                        field: "name",
                    }));
                }
                if position.code() != &code_vo
                    && uow.position_repo()?.exists_by_code(&code_vo).await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "position",
                        field: "code",
                    }));
                }

                position
                    .update_info(name_vo, code_vo, cmd.remark, operator_id, now)
                    .map_err(AppError::from)?;

                uow.position_repo()?.update(&position).await?;
                Ok(())
            })
        })
        .await
}
