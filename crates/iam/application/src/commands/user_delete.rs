use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::id::UserId;
use platform_kernel::time::Clock;
use uuid::Uuid;

pub struct UserDeleteCommand {
    pub id: Uuid,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_user_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: UserDeleteCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let user_id = UserId::from_uuid(cmd.id);
    let operator_id_vo = cmd.operator_id;

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 1. 查询用户是否存在，不存在返回 NotFound 错误
                let mut user = uow
                    .user_repo()?
                    .find_by_id(&user_id)
                    .await?
                    .ok_or(PortError::NotFound { entity: "user" })?;

                // 2. 在领域聚合根上应用软删除变更（例如标记 deleted_at、is_deleted 以及记录操作人）
                user.delete(operator_id_vo, now)?;

                // 3. 调用仓储层的 soft_delete 持久化到数据库（通常内部带版本号/乐观锁更新）
                uow.user_repo()?.soft_delete(&user).await?;
                Ok(())
            })
        })
        .await
}
