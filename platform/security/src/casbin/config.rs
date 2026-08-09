//! Casbin 模型与策略来源配置。
//!
//! 当前仅支持文件 adapter（本地 `.csv` 策略文件）。接入自定义 adapter
//! （策略存 Postgres，由 `iam-infra` 提供 role/permission 数据）时再扩展
//! 此结构，增加 `source: PolicySource` 之类的枚举，不影响现有字段。

use platform_config::ConfigMeta;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct CasbinConfig {
    /// RBAC 模型定义文件路径（`.conf`）。
    pub model_path: String,
    /// 策略文件路径（`.csv`），文件 adapter 场景使用。
    pub policy_path: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CasbinConfigError {
    #[error("model_path 不能为空")]
    EmptyModelPath,
    #[error("policy_path 不能为空")]
    EmptyPolicyPath,
}

impl ConfigMeta for CasbinConfig {
    type Error = CasbinConfigError;

    fn prefix() -> &'static str {
        "CASBIN_"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        if self.model_path.trim().is_empty() {
            return Err(CasbinConfigError::EmptyModelPath);
        }
        if self.policy_path.trim().is_empty() {
            return Err(CasbinConfigError::EmptyPolicyPath);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_model_path_rejected() {
        let cfg = CasbinConfig {
            model_path: "  ".into(),
            policy_path: "p.csv".into(),
        };
        assert!(matches!(
            cfg.validate(),
            Err(CasbinConfigError::EmptyModelPath)
        ));
    }

    #[test]
    fn empty_policy_path_rejected() {
        let cfg = CasbinConfig {
            model_path: "m.conf".into(),
            policy_path: "".into(),
        };
        assert!(matches!(
            cfg.validate(),
            Err(CasbinConfigError::EmptyPolicyPath)
        ));
    }

    #[test]
    fn load_from_rejects_missing_required_field() {
        let vars = vec![("CASBIN_MODEL_PATH".to_string(), "m.conf".to_string())];
        let result = CasbinConfig::load_from(vars);
        assert!(matches!(
            result,
            Err(platform_config::ConfigError::Load { .. })
        ));
    }

    #[test]
    fn prefix_is_casbin() {
        assert_eq!(CasbinConfig::prefix(), "CASBIN_");
    }
}

