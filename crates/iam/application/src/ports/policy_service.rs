use crate::ports::PortError;

#[async_trait::async_trait]
pub trait PolicyService: Send + Sync {
    /// 触发一次策略重新加载。
    async fn reload(&self) -> Result<(), PortError>;
}
