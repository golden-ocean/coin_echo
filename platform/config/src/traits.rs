use figment::{Figment, providers::Env};
use serde::de::DeserializeOwned;
use std::sync::Once;

static DOTENV_INIT: Once = Once::new();

/// 加载本地 `.env` 文件到进程环境变量（若存在）。
pub fn load_dotenv_if_present() {
    DOTENV_INIT.call_once(|| {
        let _ = dotenvy::dotenv();
    });
}

/// 配置加载错误。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("配置加载失败：{0}")]
    Load(#[from] figment::Error),

    #[error("配置校验失败（前缀: {prefix}）：{source}")]
    Validation {
        prefix: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub trait ConfigMeta: Sized + DeserializeOwned {
    type Error: std::error::Error + Send + Sync + 'static;

    /// 环境变量前缀（如 "SERVER_"、"DATABASE_"、"MIDDLEWARE_"）
    fn prefix() -> &'static str;

    /// 业务语义校验
    fn validate(&self) -> Result<(), Self::Error>;

    /// 自动从环境变量加载 + 校验（基于 Figment）
    fn load() -> Result<Self, ConfigError> {
        let prefix = Self::prefix();

        let figment = Figment::new().merge(
            Env::prefixed(prefix)
                .split("__")
                .map(|key| key.as_str().to_lowercase().into()),
        );

        let cfg: Self = figment.extract()?;

        cfg.validate().map_err(|source| ConfigError::Validation {
            prefix: prefix.to_string(),
            source: Box::new(source),
        })?;

        Ok(cfg)
    }

    /// 【专供测试/特定场景】从内存 KV 迭代器加载并完成自动校验
    fn load_from<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let prefix = Self::prefix();
        let mut figment = Figment::new();

        for (k, v) in iter {
            let key_str: String = k.into();
            let val_str: String = v.into();

            if let Some(stripped) = key_str.strip_prefix(prefix) {
                let figment_key = stripped.to_lowercase().replace("__", ".");

                // 尝试把 "5" 转换为 5 再 merge
                if let Ok(num) = val_str.parse::<u64>() {
                    figment = figment.merge((figment_key, num));
                } else if let Ok(b) = val_str.parse::<bool>() {
                    figment = figment.merge((figment_key, b));
                } else {
                    figment = figment.merge((figment_key, val_str));
                }
            }
        }

        let cfg: Self = figment.extract()?;

        cfg.validate().map_err(|source| ConfigError::Validation {
            prefix: prefix.to_string(),
            source: Box::new(source),
        })?;

        Ok(cfg)
    }
}
