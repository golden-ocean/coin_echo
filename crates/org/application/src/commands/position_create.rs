use org_domain::{
    error::DomainError,
    id::PositionId,
    position::{
        Position,
        value_object::{PositionCode, PositionName},
    },
};
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct PositionCreateCommand {
    pub name: String,
    pub code: String,
    pub sort: Option<i32>,
    pub remark: Option<String>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_position_create(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: PositionCreateCommand,
) -> Result<(), AppError> {
    let name_vo = PositionName::try_from(cmd.name).map_err(DomainError::from)?;
    let code_vo = PositionCode::try_from(cmd.code).map_err(DomainError::from)?;
    let new_id = PositionId::generate();
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                if uow.position_repo()?.exists_by_code(&code_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "position",
                        field: "code",
                    }));
                }
                if uow.position_repo()?.exists_by_name(&name_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "position",
                        field: "name",
                    }));
                }

                let new_position = Position::new(
                    new_id,
                    name_vo,
                    code_vo,
                    cmd.sort,
                    cmd.remark,
                    operator_id,
                    now,
                );

                uow.position_repo()?.insert(&new_position).await?;
                Ok(())
            })
        })
        .await
}
