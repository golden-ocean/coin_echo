use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::id::PermissionId;
use platform_kernel::time::Clock;
use uuid::Uuid;

pub struct PermissionDeleteCommand {
    pub id: Uuid,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_permission_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: PermissionDeleteCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let permission_id = PermissionId::from_uuid(cmd.id);
    let operator_id_vo = cmd.operator_id;

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 1. 查询权限是否存在
                let mut permission = uow
                    .permission_repo()?
                    .find_by_id(&permission_id)
                    .await?
                    .ok_or(PortError::NotFound {
                        entity: "permission",
                    })?;

                // 2. 存在子权限时禁止删除，避免产生挂在不存在父节点下的孤儿数据
                if uow.permission_repo()?.has_children(&permission_id).await? {
                    return Err(AppError::from(PortError::HasChildren {
                        entity: "permission",
                    }));
                }

                // 3. 领域聚合根应用软删除（内置/已删除拦截交由领域方法内部处理）
                permission.delete(operator_id_vo, now)?;

                // 4. 落库
                uow.permission_repo()?.soft_delete(&permission).await?;
                Ok(())
            })
        })
        .await
}
