use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::{
    error::DomainError,
    id::PermissionId,
    permission::{
        Permission,
        value_object::{ApiMethod, PermissionCode, PermissionKind, PermissionName},
    },
};
use platform_kernel::time::Clock;
use uuid::Uuid;

pub struct PermissionCreateCommand {
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub kind: String,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub api_method: Option<String>,
    pub api_path: Option<String>,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_permission_create(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: PermissionCreateCommand,
) -> Result<(), AppError> {
    let name_vo = PermissionName::try_from(cmd.name).map_err(DomainError::from)?;
    let code_vo = PermissionCode::try_from(cmd.code).map_err(DomainError::from)?;
    let kind_vo = PermissionKind::try_from(cmd.kind).map_err(DomainError::from)?;
    let api_method_vo = cmd
        .api_method
        .as_deref()
        .map(ApiMethod::try_from)
        .transpose()
        .map_err(DomainError::from)?;
    let parent_id_vo = cmd.parent_id.map(PermissionId::from_uuid);
    let new_permission_id = PermissionId::generate();
    let operator_id_vo = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 1. 若指定了父级，先确认父级权限存在，避免挂到一个不存在的节点下
                if let Some(pid) = parent_id_vo {
                    uow.permission_repo()?
                        .find_by_id(&pid)
                        .await?
                        .ok_or(PortError::NotFound {
                            entity: "permission",
                        })?;
                }

                // 2. 唯一性校验：code / name
                if uow.permission_repo()?.exists_by_code(&code_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "permission",
                        field: "code",
                    }));
                }
                if uow.permission_repo()?.exists_by_name(&name_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "permission",
                        field: "name",
                    }));
                }

                // 3. 构造聚合根（内部会做 kind 与附属字段的一致性校验）
                let new_permission = Permission::new(
                    new_permission_id,
                    parent_id_vo,
                    name_vo,
                    code_vo,
                    kind_vo,
                    cmd.route_path,
                    cmd.component,
                    cmd.icon,
                    api_method_vo,
                    cmd.api_path,
                    cmd.remark,
                    cmd.sort,
                    None,
                    operator_id_vo,
                    now,
                )?;

                uow.permission_repo()?.insert(&new_permission).await?;
                Ok(())
            })
        })
        .await
}
