use org_domain::{
    error::DomainError,
    id::OrganizationId,
    organization::value_object::{OrganizationCode, OrganizationName},
};
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct OrganizationUpdateCommand {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub remark: Option<String>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_organization_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: OrganizationUpdateCommand,
) -> Result<(), AppError> {
    let id = OrganizationId::from(cmd.id);
    let name_vo = OrganizationName::try_from(cmd.name).map_err(DomainError::from)?;
    let code_vo = OrganizationCode::try_from(cmd.code).map_err(DomainError::from)?;
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let mut org = uow
                    .organization_repo()?
                    .find_by_id(&id)
                    .await?
                    .ok_or_else(|| {
                        AppError::from(PortError::NotFound {
                            entity: "organization",
                        })
                    })?;

                if org.name() != &name_vo
                    && uow.organization_repo()?.exists_by_name(&name_vo).await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "organization",
                        field: "name",
                    }));
                }
                if org.code() != &code_vo
                    && uow.organization_repo()?.exists_by_code(&code_vo).await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "organization",
                        field: "code",
                    }));
                }

                org.update_info(
                    name_vo,
                    code_vo,
                    cmd.contact,
                    cmd.phone,
                    cmd.email,
                    cmd.remark,
                    operator_id,
                    now,
                )
                .map_err(AppError::from)?;

                uow.organization_repo()?.update(&org).await?;
                Ok(())
            })
        })
        .await
}
