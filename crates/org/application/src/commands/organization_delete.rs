use org_domain::id::OrganizationId;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{MembershipChecker, PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct OrganizationDeleteCommand {
    pub id: Uuid,
}

pub async fn handle_organization_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    membership_checker: &dyn MembershipChecker,
    cmd: OrganizationDeleteCommand,
) -> Result<(), AppError> {
    let id = OrganizationId::from(cmd.id);

    // has_members 依赖跨模块查询（iam_user 表），不在事务内查询，
    // 因为组织删除本身不需要和“是否有用户”这件事共享同一个事务边界
    let has_members = membership_checker
        .has_users_in_organization(id.into())
        .await?;

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let org = uow
                    .organization_repo()?
                    .find_by_id(&id)
                    .await?
                    .ok_or_else(|| {
                        AppError::from(PortError::NotFound {
                            entity: "organization",
                        })
                    })?;

                let has_children = uow.organization_repo()?.exists_children(&id).await?;

                org.ensure_deletable(has_children, has_members)
                    .map_err(AppError::from)?;

                uow.organization_repo()?.soft_delete(&org).await?;
                Ok(())
            })
        })
        .await
}
