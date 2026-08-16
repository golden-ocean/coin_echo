use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::{
    error::DomainError,
    id::PermissionId,
    permission::value_object::{ApiMethod, PermissionCode, PermissionKind, PermissionName},
};
use platform_kernel::time::Clock;
use std::str::FromStr;
use uuid::Uuid;

pub struct PermissionUpdateCommand {
    pub id: Uuid,
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

pub async fn handle_permission_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: PermissionUpdateCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let permission_id = PermissionId::from_uuid(cmd.id);
    let name_vo = PermissionName::from_str(&cmd.name).map_err(DomainError::from)?;
    let code_vo = PermissionCode::from_str(&cmd.code).map_err(DomainError::from)?;
    let kind_vo = PermissionKind::from_str(&cmd.kind).map_err(DomainError::from)?;
    let api_method_vo = cmd
        .api_method
        .as_deref()
        .map(ApiMethod::from_str)
        .transpose()
        .map_err(DomainError::from)?;
    let operator_id_vo = cmd.operator_id;

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 1. 先查询权限是否存在，不存在直接返回 NotFound，避免后续无意义的唯一性查询
                let mut permission = uow
                    .permission_repo()?
                    .find_by_id(&permission_id)
                    .await?
                    .ok_or(PortError::NotFound {
                        entity: "permission",
                    })?;

                // 2. 只有当 code / name 确实发生变化时，才需要查唯一性
                //    —— 避免值未变时的无意义查询，同时排除"查到自己"导致的误判冲突
                if permission.code() != &code_vo
                    && uow.permission_repo()?.exists_by_code(&code_vo).await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "permission",
                        field: "code",
                    }));
                }
                if permission.name() != &name_vo
                    && uow.permission_repo()?.exists_by_name(&name_vo).await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "permission",
                        field: "name",
                    }));
                }

                // 3. 在已查出的聚合根上应用变更
                //    （领域方法内部负责 kind/附属字段一致性校验、递增 version、更新 audit_meta 等）
                permission.update_info(
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
                    operator_id_vo,
                    now,
                )?;

                // 4. 落库（repo.update 内部通过 version 做乐观锁校验）
                uow.permission_repo()?.update(&permission).await?;
                Ok(())
            })
        })
        .await
}
