use org_domain::{
    id::PositionId,
    position::{
        Position,
        value_object::{PositionCode, PositionName},
    },
};

use crate::ports::error::PortError;

#[async_trait::async_trait]
pub trait PositionRepository: Send + Sync {
    async fn insert(&mut self, position: &Position) -> Result<(), PortError>;
    async fn update(&mut self, position: &Position) -> Result<(), PortError>;
    async fn soft_delete(&mut self, position: &Position) -> Result<(), PortError>;

    async fn find_by_id(&mut self, id: &PositionId) -> Result<Option<Position>, PortError>;
    async fn find_by_code(&mut self, code: &PositionCode) -> Result<Option<Position>, PortError>;
    async fn find_by_name(&mut self, name: &PositionName) -> Result<Option<Position>, PortError>;

    async fn exists_by_code(&mut self, code: &PositionCode) -> Result<bool, PortError>;
    async fn exists_by_name(&mut self, name: &PositionName) -> Result<bool, PortError>;
}
