use sys_domain::id::DictionaryItemId;
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct DictionaryItemDeleteCommand {
    pub id: Uuid,
}

pub async fn handle_dictionary_item_delete(
    uow_factory: &dyn UnitOfWorkFactory,
    cmd: DictionaryItemDeleteCommand,
) -> Result<(), AppError> {
    let id = DictionaryItemId::from_uuid(cmd.id);

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let item = uow
                    .dict_item_repo()?
                    .find_by_id(&id)
                    .await?
                    .ok_or_else(|| {
                        AppError::from(PortError::NotFound {
                            entity: "dictionary_item",
                        })
                    })?;

                item.ensure_deletable().map_err(AppError::from)?;

                uow.dict_item_repo()?.delete(&item).await?;
                Ok(())
            })
        })
        .await
}
