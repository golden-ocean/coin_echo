use std::str::FromStr;

use iam_domain::{
    error::DomainError,
    id::RoleId,
    role::{
        Role,
        value_object::{RoleCode, RoleName},
    },
};

use platform_kernel::time::Clock;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct RoleCreateCommand {
    pub name: String,
    pub code: String,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_role_create(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: RoleCreateCommand,
) -> Result<(), AppError> {
    let name_vo = RoleName::from_str(&cmd.name).map_err(DomainError::from)?;
    let code_vo = RoleCode::from_str(&cmd.code).map_err(DomainError::from)?;
    let new_role_id = RoleId::generate();
    let operator_id_vo = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                if uow.role_repo()?.exists_by_code(&code_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "role",
                        field: "code",
                    }));
                }
                if uow.role_repo()?.exists_by_name(&name_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "role",
                        field: "name",
                    }));
                }

                let new_role = Role::new(
                    new_role_id,
                    name_vo,
                    code_vo,
                    cmd.remark,
                    cmd.sort,
                    None,
                    operator_id_vo,
                    now,
                );
                uow.role_repo()?.insert(&new_role).await?;
                Ok(())
            })
        })
        .await

    // execute_in_uow(uow_factory, move |uow| {
    //     Box::pin(async move {
    //         if uow.role_repo()?.exists_by_code(&code_vo).await? {
    //             return Err(AppError::from(PortError::UniqueConflict {
    //                 entity: "role",
    //                 field: "code",
    //             }));
    //         }
    //         if uow.role_repo()?.exists_by_name(&name_vo).await? {
    //             return Err(AppError::from(PortError::UniqueConflict {
    //                 entity: "role",
    //                 field: "name",
    //             }));
    //         }
    //         let new_role = Role::new(
    //             new_role_id,
    //             name_vo,
    //             code_vo,
    //             cmd.remark,
    //             cmd.sort,
    //             None,
    //             operator_id_vo,
    //             now,
    //         );
    //         uow.role_repo()?.insert(&new_role).await?;
    //         Ok(())
    //     })
    // })
    // .await
}
