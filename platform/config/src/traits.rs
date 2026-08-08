use std::sync::Once;

use serde::de::DeserializeOwned;

static DOTENV_INIT: Once = Once::new();

/// 加载本地 `.env` 文件到进程环境变量（若存在）。
/// 建议在 main() 函数的最开头调用一次。
pub fn load_dotenv_if_present() {
    DOTENV_INIT.call_once(|| {
        let _ = dotenvy::dotenv();
    });
}

/// 配置加载错误。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("加载配置失败（前缀: {prefix}）：{source}")]
    Load {
        prefix: String,
        #[source]
        source: envy::Error,
    },
    /// 语义校验失败（端口为 0、超时时间不合法等）
    #[error("配置校验失败（前缀: {prefix}）：{source}")]
    Validation {
        prefix: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

pub trait ConfigMeta: Sized {
    type Error: std::error::Error + Send + Sync + 'static;

    /// 环境变量前缀（如 "SERVER_"、"DATABASE_"）
    fn prefix() -> &'static str;

    /// 业务语义校验
    fn validate(&self) -> Result<(), Self::Error>;

    /// 自动加载 + 校验（统一捕获并附带 prefix 信息）
    fn load() -> Result<Self, ConfigError>
    where
        Self: serde::de::DeserializeOwned,
    {
        let prefix = Self::prefix();

        // 1. 反序列化：若失败，记录具体的 prefix
        let cfg: Self = envy::prefixed(prefix)
            .from_env()
            .map_err(|source| ConfigError::Load {
                prefix: prefix.to_string(),
                source,
            })?;

        // 2. 自自我校验：若失败，将具体业务 Error 打包并附带 prefix
        cfg.validate().map_err(|source| ConfigError::Validation {
            prefix: prefix.to_string(),
            source: Box::new(source),
        })?;

        Ok(cfg)
    }

    /// 【专供测试/特定场景】从内存 KV 迭代器加载并完成自动校验
    fn load_from<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        Self: DeserializeOwned,
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let prefix = Self::prefix();
        let cfg: Self = envy::prefixed(prefix)
            .from_iter(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
            .map_err(|source| ConfigError::Load {
                prefix: prefix.to_string(),
                source,
            })?;

        cfg.validate().map_err(|source| ConfigError::Validation {
            prefix: prefix.to_string(),
            source: Box::new(source),
        })?;

        Ok(cfg)
    }
}
