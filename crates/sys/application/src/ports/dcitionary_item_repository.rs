use sys_domain::{
    dictionary::{
        DictionaryItem,
        value_object::{DictionaryItemLabel, DictionaryItemValue},
    },
    id::{DictionaryId, DictionaryItemId},
};

use crate::ports::PortError;

#[async_trait::async_trait]
pub trait DictionaryItemRepository: Send + Sync {
    async fn insert(&mut self, item: &DictionaryItem) -> Result<(), PortError>;
    async fn update(&mut self, item: &DictionaryItem) -> Result<(), PortError>;
    async fn delete(&mut self, item: &DictionaryItem) -> Result<(), PortError>;

    async fn find_by_id(
        &mut self,
        id: &DictionaryItemId,
    ) -> Result<Option<DictionaryItem>, PortError>;

    /// 根据 字典ID + Label 唯一查询
    async fn find_by_dict_id_and_label(
        &mut self,
        dictionary_id: &DictionaryId,
        label: &DictionaryItemLabel,
    ) -> Result<Option<DictionaryItem>, PortError>;

    /// 根据 字典ID + Value 唯一查询
    async fn find_by_dict_id_and_value(
        &mut self,
        dictionary_id: &DictionaryId,
        value: &DictionaryItemValue,
    ) -> Result<Option<DictionaryItem>, PortError>;

    /// 查询某个字典下的所有字典项列表
    async fn find_by_dictionary_id(
        &mut self,
        dictionary_id: &DictionaryId,
    ) -> Result<Vec<DictionaryItem>, PortError>;

    async fn exists_by_dictionary_id(
        &mut self,
        dictionary_id: &DictionaryId,
    ) -> Result<bool, PortError>;

    /// 校验相同字典下 Label 是否重复
    async fn exists_by_dict_id_and_label(
        &mut self,
        dictionary_id: &DictionaryId,
        label: &DictionaryItemLabel,
    ) -> Result<bool, PortError>;

    /// 校验相同字典下 Value 是否重复
    async fn exists_by_dict_id_and_value(
        &mut self,
        dictionary_id: &DictionaryId,
        value: &DictionaryItemValue,
    ) -> Result<bool, PortError>;
}
