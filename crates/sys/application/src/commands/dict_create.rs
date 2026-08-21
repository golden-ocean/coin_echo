use std::str::FromStr;

use platform_kernel::time::Clock;
use sys_domain::{
    dictionary::{
        Dictionary,
        value_object::{DictionaryCode, DictionaryName},
    },
    error::DomainError,
    id::DictionaryId,
};
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct DictionaryCreateCommand {
    pub name: String,
    pub code: String,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_dictionary_create(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: DictionaryCreateCommand,
) -> Result<(), AppError> {
    let name_vo = DictionaryName::from_str(&cmd.name).map_err(DomainError::from)?;
    let code_vo = DictionaryCode::from_str(&cmd.code).map_err(DomainError::from)?;
    let new_id = DictionaryId::generate();
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                if uow.dict_repo()?.exists_by_code(&code_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "dictionary",
                        field: "code",
                    }));
                }
                if uow.dict_repo()?.exists_by_name(&name_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "dictionary",
                        field: "name",
                    }));
                }

                let new_dictionary = Dictionary::new(
                    new_id,
                    name_vo,
                    code_vo,
                    operator_id,
                    cmd.sort,
                    cmd.remark,
                    now,
                );

                uow.dict_repo()?.insert(&new_dictionary).await?;
                Ok(())
            })
        })
        .await
}
