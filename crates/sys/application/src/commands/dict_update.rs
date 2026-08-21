use platform_kernel::time::Clock;
use sys_domain::{dictionary::value_object::DictionaryName, error::DomainError, id::DictionaryId};
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct DictionaryUpdateCommand {
    pub id: Uuid,
    pub name: String,
    pub remark: Option<String>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_dictionary_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: DictionaryUpdateCommand,
) -> Result<(), AppError> {
    let id = DictionaryId::from(cmd.id);
    let name_vo = DictionaryName::try_from(&cmd.name).map_err(DomainError::from)?;
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let mut dict = uow.dict_repo()?.find_by_id(&id).await?.ok_or_else(|| {
                    AppError::from(PortError::NotFound {
                        entity: "dictionary",
                    })
                })?;

                // name 变更时才需要查重；未改名则跳过，避免误判"和自己重名"
                if dict.name() != &name_vo && uow.dict_repo()?.exists_by_name(&name_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "dictionary",
                        field: "name",
                    }));
                }

                dict.update_info(name_vo, cmd.remark, operator_id, now)
                    .map_err(AppError::from)?;

                uow.dict_repo()?.update(&dict).await?;
                Ok(())
            })
        })
        .await
}
