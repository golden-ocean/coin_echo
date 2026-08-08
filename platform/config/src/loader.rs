use std::sync::Once;

use serde::de::DeserializeOwned;

/// 确保 `.env` 文件在整个进程生命周期中仅加载一次。
static DOTENV_INIT: Once = Once::new();

/// 配置加载错误。
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("加载配置失败（前缀: {prefix}）：{source}")]
    Load {
        prefix: String,
        #[source]
        source: envy::Error,
    },
}

/// 从环境变量加载配置，按前缀隔离。
pub fn load_prefixed<T: DeserializeOwned>(prefix: &str) -> Result<T, ConfigError> {
    envy::prefixed(prefix)
        .from_env::<T>()
        .map_err(|source| ConfigError::Load {
            prefix: prefix.to_string(),
            source,
        })
}

/// 加载本地 `.env` 文件到进程环境变量（若存在）。
///
/// - 本地开发：读取 `.env` 方便调试。
/// - 生产环境：不依赖此文件，环境变量由 Container / K8s 直接注入。
/// - `Once` 保证：`dotenvy::dotenv()` 本身是幂等的（重复调用不会导致
///   环境变量错乱），这里用 `Once` 纯粹是为了避免重复的文件 IO，以及
///   让"整个进程只应加载一次"这个调用约定在类型层面显式表达出来，
///   而不是依赖调用方自觉只调一次。
pub fn load_dotenv_if_present() {
    DOTENV_INIT.call_once(|| {
        let _ = dotenvy::dotenv();
    });
}

/// 纯函数入口：从任意内存 KV 迭代器加载（专供单元测试/特定内存场景）
pub fn load_prefixed_from<T, I, K, V>(prefix: &str, iter: I) -> Result<T, ConfigError>
where
    T: DeserializeOwned,
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: Into<String>,
{
    // 将 envy 的调用封装在 platform_config 内部
    envy::prefixed(prefix)
        .from_iter(iter.into_iter().map(|(k, v)| (k.into(), v.into())))
        .map_err(|source| ConfigError::Load {
            prefix: prefix.to_string(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Sample {
        name: String,
        port: u16,
    }

    #[test]
    fn error_message_includes_prefix_for_diagnosis() {
        // 触发必填项缺失错误，确认 prefix 正确包含在错误日志中
        let result: Result<Sample, _> = load_prefixed("MISSING_PREFIX_");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("MISSING_PREFIX_"));
    }

    #[test]
    fn load_dotenv_if_present_is_idempotent_and_does_not_panic() {
        // 不断言具体环境变量效果（依赖当前目录是否存在 .env，跑测试的环境
        // 不确定），只验证多次调用是安全的、不会因为重复调用而出错。
        load_dotenv_if_present();
        load_dotenv_if_present();
        load_dotenv_if_present();
    }
}
