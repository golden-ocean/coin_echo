use sys_domain::{
    dictionary::{
        Dictionary,
        value_object::{DictionaryCode, DictionaryName},
    },
    id::DictionaryId,
};

use crate::ports::PortError;

#[async_trait::async_trait]
pub trait DictionaryRepository: Send + Sync {
    async fn insert(&mut self, dictionary: &Dictionary) -> Result<(), PortError>;
    async fn update(&mut self, dictionary: &Dictionary) -> Result<(), PortError>;
    async fn delete(&mut self, dictionary: &Dictionary) -> Result<(), PortError>;

    async fn find_by_id(&mut self, id: &DictionaryId) -> Result<Option<Dictionary>, PortError>;
    async fn find_by_code(
        &mut self,
        code: &DictionaryCode,
    ) -> Result<Option<Dictionary>, PortError>;
    async fn find_by_name(
        &mut self,
        name: &DictionaryName,
    ) -> Result<Option<Dictionary>, PortError>;

    async fn exists_by_code(&mut self, code: &DictionaryCode) -> Result<bool, PortError>;
    async fn exists_by_name(&mut self, name: &DictionaryName) -> Result<bool, PortError>;
}
