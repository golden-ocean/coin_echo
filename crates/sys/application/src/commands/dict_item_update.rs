use platform_kernel::time::Clock;
use sys_domain::{
    dictionary::value_object::{DictionaryItemColor, DictionaryItemLabel},
    error::DomainError,
    id::DictionaryItemId,
};
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct DictionaryItemUpdateCommand {
    pub id: Uuid,
    pub label: String,
    pub color: Option<String>,
    pub remark: Option<String>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_dictionary_item_update(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: DictionaryItemUpdateCommand,
) -> Result<(), AppError> {
    let id = DictionaryItemId::from(cmd.id);
    let label_vo = DictionaryItemLabel::try_from(cmd.label).map_err(DomainError::from)?;
    let color_vo = cmd
        .color
        .map(|c| DictionaryItemColor::try_from(c))
        .transpose()
        .map_err(DomainError::from)?;
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                let mut item = uow
                    .dict_item_repo()?
                    .find_by_id(&id)
                    .await?
                    .ok_or_else(|| {
                        AppError::from(PortError::NotFound {
                            entity: "dictionary_item",
                        })
                    })?;

                // label 变更时才需要在同一字典下查重
                if item.label() != &label_vo
                    && uow
                        .dict_item_repo()?
                        .exists_by_dict_id_and_label(&item.dictionary_id(), &label_vo)
                        .await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "dictionary_item",
                        field: "label",
                    }));
                }

                item.update_info(label_vo, color_vo, cmd.remark, operator_id, now)
                    .map_err(AppError::from)?;

                uow.dict_item_repo()?.update(&item).await?;
                Ok(())
            })
        })
        .await
}
