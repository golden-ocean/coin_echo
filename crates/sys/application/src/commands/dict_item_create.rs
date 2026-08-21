use uuid::Uuid;

use platform_kernel::time::Clock;
use sys_domain::{
    dictionary::{
        DictionaryItem,
        value_object::{DictionaryItemColor, DictionaryItemLabel, DictionaryItemValue},
    },
    error::DomainError,
    id::{DictionaryId, DictionaryItemId},
};

use crate::{
    error::AppError,
    ports::{PortError, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct DictionaryItemCreateCommand {
    pub dictionary_id: Uuid,
    pub label: String,
    pub value: String,
    pub color: Option<String>,
    pub remark: Option<String>,
    pub sort: Option<i32>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_dictionary_item_create(
    uow_factory: &dyn UnitOfWorkFactory,
    clock: &dyn Clock,
    cmd: DictionaryItemCreateCommand,
) -> Result<(), AppError> {
    let dictionary_id = DictionaryId::from(cmd.dictionary_id);
    let label_vo = DictionaryItemLabel::try_from(cmd.label).map_err(DomainError::from)?;
    let value_vo = DictionaryItemValue::try_from(cmd.value).map_err(DomainError::from)?;
    let color_vo = cmd
        .color
        .map(|c| DictionaryItemColor::try_from(c))
        .transpose()
        .map_err(DomainError::from)?;
    let new_id = DictionaryItemId::generate();
    let operator_id = cmd.operator_id;
    let now = clock.now();

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                // 父聚合存在性校验：字典项必须挂靠在真实存在的字典下
                if uow.dict_repo()?.find_by_id(&dictionary_id).await?.is_none() {
                    return Err(AppError::from(PortError::NotFound {
                        entity: "dictionary",
                    }));
                }

                if uow
                    .dict_item_repo()?
                    .exists_by_dict_id_and_label(&dictionary_id, &label_vo)
                    .await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "dictionary_item",
                        field: "label",
                    }));
                }
                if uow
                    .dict_item_repo()?
                    .exists_by_dict_id_and_value(&dictionary_id, &value_vo)
                    .await?
                {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "dictionary_item",
                        field: "value",
                    }));
                }

                let new_item = DictionaryItem::new(
                    new_id,
                    dictionary_id,
                    label_vo,
                    value_vo,
                    color_vo,
                    operator_id,
                    cmd.sort,
                    cmd.remark,
                    now,
                );

                uow.dict_item_repo()?.insert(&new_item).await?;
                Ok(())
            })
        })
        .await
}
