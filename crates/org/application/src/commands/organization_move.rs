use org_domain::id::OrganizationId;
use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct OrganizationMoveCommand {
    pub id: Uuid,
    pub new_parent_id: Option<Uuid>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_organization_move(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: OrganizationMoveCommand,
) -> Result<(), AppError> {
    let id = OrganizationId::from(cmd.id);
    let new_parent_id = cmd.new_parent_id.map(OrganizationId::from);
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

                // 新父节点存在性校验
                if let Some(pid) = new_parent_id {
                    if uow.organization_repo()?.find_by_id(&pid).await?.is_none() {
                        return Err(AppError::from(PortError::NotFound {
                            entity: "organization",
                        }));
                    }
                }

                // 完整的“不能移到自己子孙节点下”校验需要遍历整棵子树，
                // 这里先做 domain 层已有的“不能设自己为父节点”这一层基础校验；
                // 更完整的子树校验建议在 API/application 层调用 Query 层的
                // build_organization_tree 拿到子树 id 集合后再传入这里做二次校验，
                // 或者在 domain 层新增一个接收 descendant_ids 的重载方法
                org.move_to(new_parent_id, operator_id, now)
                    .map_err(AppError::from)?;

                uow.organization_repo()?.update(&org).await?;
                Ok(())
            })
        })
        .await
}
