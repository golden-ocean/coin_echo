use std::sync::Arc;

use iam_application::ports::{PolicyService, PortError};
use platform_security::casbin::CasbinEnforcer;

pub struct CasbinPolicyService {
    enforcer: Arc<CasbinEnforcer>,
}

impl CasbinPolicyService {
    pub fn new(enforcer: Arc<CasbinEnforcer>) -> Self {
        Self { enforcer }
    }
}

#[async_trait::async_trait]
impl PolicyService for CasbinPolicyService {
    async fn reload(&self) -> Result<(), PortError> {
        self.enforcer
            .reload_policy()
            .await
            .map_err(|e| PortError::Infrastructure(e.to_string()))
    }
}
