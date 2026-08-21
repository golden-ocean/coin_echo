use sys_domain::id::DictionaryId;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct DictionaryDeleteCommand {
    pub id: Uuid,
}

pub async fn handle_dictionary_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    cmd: DictionaryDeleteCommand,
) -> Result<(), AppError> {
    let id = DictionaryId::from(cmd.id);

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let dict = uow.dict_repo()?.find_by_id(&id).await?.ok_or_else(|| {
                    AppError::from(PortError::NotFound {
                        entity: "dictionary",
                    })
                })?;

                // 是否还有子项由仓储轻量 EXISTS 查询给出，不拉取完整数据
                let has_items = uow.dict_item_repo()?.exists_by_dictionary_id(&id).await?;

                dict.ensure_deletable(has_items).map_err(AppError::from)?;

                uow.dict_repo()?.delete(&dict).await?;
                Ok(())
            })
        })
        .await
}
