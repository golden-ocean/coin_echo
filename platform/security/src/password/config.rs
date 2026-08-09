//! Argon2id 参数配置。
//!
//! 默认值取 OWASP 密码存储备忘录推荐的 Argon2id 基线（内存 19 MiB、
//! 2 次迭代、1 并行度），在服务器环境下平衡安全性与哈希耗时。

use platform_config::ConfigMeta;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct PasswordConfig {
    /// 内存成本（KiB）。
    #[serde(default = "PasswordConfig::default_memory_kib")]
    pub memory_kib: u32,
    /// 迭代次数。
    #[serde(default = "PasswordConfig::default_iterations")]
    pub iterations: u32,
    /// 并行度。
    #[serde(default = "PasswordConfig::default_parallelism")]
    pub parallelism: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum PasswordConfigError {
    #[error("memory_kib 过低（{0} KiB），至少需要 19456 KiB（19 MiB），否则易被硬件加速爆破")]
    MemoryTooLow(u32),
    #[error("iterations 必须至少为 1，当前为 {0}")]
    NonPositiveIterations(u32),
    #[error("parallelism 必须至少为 1，当前为 {0}")]
    NonPositiveParallelism(u32),
}

impl PasswordConfig {
    const MIN_MEMORY_KIB: u32 = 19 * 1024;

    const fn default_memory_kib() -> u32 {
        Self::MIN_MEMORY_KIB
    }

    const fn default_iterations() -> u32 {
        2
    }

    const fn default_parallelism() -> u32 {
        1
    }
}

impl ConfigMeta for PasswordConfig {
    type Error = PasswordConfigError;

    fn prefix() -> &'static str {
        "PASSWORD_"
    }

    fn validate(&self) -> Result<(), Self::Error> {
        if self.memory_kib < Self::MIN_MEMORY_KIB {
            return Err(PasswordConfigError::MemoryTooLow(self.memory_kib));
        }
        if self.iterations == 0 {
            return Err(PasswordConfigError::NonPositiveIterations(self.iterations));
        }
        if self.parallelism == 0 {
            return Err(PasswordConfigError::NonPositiveParallelism(
                self.parallelism,
            ));
        }
        Ok(())
    }
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            memory_kib: Self::default_memory_kib(),
            iterations: Self::default_iterations(),
            parallelism: Self::default_parallelism(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        assert!(PasswordConfig::default().validate().is_ok());
    }

    #[test]
    fn memory_below_owasp_minimum_rejected() {
        let cfg = PasswordConfig {
            memory_kib: 8192,
            ..PasswordConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PasswordConfigError::MemoryTooLow(8192))
        ));
    }

    #[test]
    fn zero_iterations_rejected() {
        let cfg = PasswordConfig {
            iterations: 0,
            ..PasswordConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PasswordConfigError::NonPositiveIterations(0))
        ));
    }

    #[test]
    fn zero_parallelism_rejected() {
        let cfg = PasswordConfig {
            parallelism: 0,
            ..PasswordConfig::default()
        };
        assert!(matches!(
            cfg.validate(),
            Err(PasswordConfigError::NonPositiveParallelism(0))
        ));
    }

    #[test]
    fn load_from_applies_defaults_when_no_vars_present() {
        let cfg = PasswordConfig::load_from(Vec::<(String, String)>::new()).unwrap();
        assert_eq!(cfg.memory_kib, 19 * 1024);
        assert_eq!(cfg.iterations, 2);
    }

    #[test]
    fn load_from_rejects_semantically_invalid_memory_kib() {
        let vars = vec![("PASSWORD_MEMORY_KIB".to_string(), "1024".to_string())];
        let result = PasswordConfig::load_from(vars);
        assert!(matches!(
            result,
            Err(platform_config::ConfigError::Validation { .. })
        ));
    }

    #[test]
    fn prefix_is_password() {
        assert_eq!(PasswordConfig::prefix(), "PASSWORD_");
    }
}

