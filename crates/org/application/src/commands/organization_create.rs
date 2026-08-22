use org_domain::{
    error::DomainError,
    id::OrganizationId,
    organization::{
        Organization,
        value_object::{OrganizationCode, OrganizationName},
    },
};
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct OrganizationCreateCommand {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub sort: Option<i32>,
    pub remark: Option<String>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_organization_create(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: OrganizationCreateCommand,
) -> Result<(), AppError> {
    let name_vo = OrganizationName::try_from(cmd.name).map_err(DomainError::from)?;
    let code_vo = OrganizationCode::try_from(cmd.code).map_err(DomainError::from)?;
    let new_id = OrganizationId::generate();
    let parent_id = cmd.parent_id.map(OrganizationId::from);
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                if uow.organization_repo()?.exists_by_code(&code_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "organization",
                        field: "code",
                    }));
                }
                if uow.organization_repo()?.exists_by_name(&name_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "organization",
                        field: "name",
                    }));
                }

                // 父组织存在性校验（若指定了 parent_id）
                if let Some(pid) = parent_id {
                    if uow.organization_repo()?.find_by_id(&pid).await?.is_none() {
                        return Err(AppError::from(PortError::NotFound {
                            entity: "organization",
                        }));
                    }
                }

                let new_org = Organization::new(
                    new_id,
                    parent_id,
                    name_vo,
                    code_vo,
                    cmd.contact,
                    cmd.phone,
                    cmd.email,
                    cmd.sort,
                    cmd.remark,
                    operator_id,
                    now,
                );

                uow.organization_repo()?.insert(&new_org).await?;
                Ok(())
            })
        })
        .await
}
