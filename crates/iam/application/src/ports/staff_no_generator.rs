use iam_domain::user::value_object::StaffNo;

use crate::ports::PortError;

#[async_trait::async_trait]
pub trait StaffNoGenerator: Send + Sync {
    /// 生成下一个唯一的工号
    async fn generate(&self) -> Result<StaffNo, PortError>;
}
