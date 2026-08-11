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
    use platform_config::ConfigError;

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

    // ---- 加载测试：ConfigMeta::load_from 返回 Result ----

    #[test]
    fn load_from_succeeds_with_all_required_fields() {
        let cfg = CasbinConfig::load_from(vec![
            ("CASBIN_MODEL_PATH", "m.conf"),
            ("CASBIN_POLICY_PATH", "p.csv"),
        ])
        .unwrap();
        assert_eq!(cfg.model_path, "m.conf");
        assert_eq!(cfg.policy_path, "p.csv");
    }

    /// 必填字段缺失（缺 policy_path）→ Load 错误
    #[test]
    fn load_from_fails_on_missing_required_field() {
        let result = CasbinConfig::load_from(vec![("CASBIN_MODEL_PATH", "m.conf")]);
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    /// 反序列化成功但语义非法（空白路径）→ Validation 错误
    #[test]
    fn load_from_rejects_blank_paths() {
        let result = CasbinConfig::load_from(vec![
            ("CASBIN_MODEL_PATH", "  "),
            ("CASBIN_POLICY_PATH", "p.csv"),
        ]);
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }

    /// 变量名大小写不敏感
    #[test]
    fn env_keys_are_case_insensitive() {
        let cfg = CasbinConfig::load_from(vec![
            ("casbin_model_path", "m.conf"),
            ("casbin_policy_path", "p.csv"),
        ])
        .unwrap();
        assert_eq!(cfg.model_path, "m.conf");
    }

    /// 非 CASBIN_ 前缀的键被忽略 → 必填缺失报 Load 错误
    #[test]
    fn non_prefixed_keys_are_ignored() {
        let result = CasbinConfig::load_from(vec![("RBAC_MODEL_PATH", "m.conf")]);
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    #[test]
    fn prefix_is_casbin() {
        assert_eq!(CasbinConfig::prefix(), "CASBIN_");
    }
}
