use figment::Figment;
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
        prefix: &'static str,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// 从环境变量加载配置的通用能力。
///
/// # 变量命名规则
///
/// - 变量名 = `前缀 + 字段路径`，如 `SERVER_PORT`、`MIDDLEWARE_TIMEOUT_SECS`。
/// - **嵌套层级用双下划线 `__`**：`cors.allowed_origins` 对应
///   `MIDDLEWARE_CORS__ALLOWED_ORIGINS`；平铺字段用单下划线，不要写成
///   `MIDDLEWARE_REQUEST_ID__ENABLED`（那会被解析成 `request_id.enabled`
///   这个不存在的嵌套路径而静默忽略）。
/// - 值交给 figment 解析（与 `Env` 相同）：`true`/`false` → bool、整数 →
///   数值、含小数点 → 浮点、其余 → 字符串。
pub trait ConfigMeta: Sized + DeserializeOwned {
    type Error: std::error::Error + Send + Sync + 'static;

    /// 环境变量前缀（如 "SERVER_"、"DATABASE_"、"MIDDLEWARE_"）
    fn prefix() -> &'static str;

    /// 业务语义校验
    fn validate(&self) -> Result<(), Self::Error>;

    /// 生产路径：从真实环境变量加载 + 校验。
    ///
    /// 直接委托 [`ConfigMeta::load_from`]——生产与测试共用同一实现，
    /// 不存在两套逻辑漂移的可能。
    fn load() -> Result<Self, ConfigError> {
        Self::load_from(std::env::vars_os().map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.to_string_lossy().into_owned(),
            )
        }))
    }

    /// 唯一加载实现：前缀剥离 → 小写 → `__` 表示嵌套 → figment 解析值。
    fn load_from<I, K, V>(iter: I) -> Result<Self, ConfigError>
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let prefix = Self::prefix();

        let mut figment = Figment::new();
        for (k, v) in iter {
            let (k, v) = (k.into(), v.into());

            // 先绑定再借用：`to_ascii_uppercase()` 的临时 String 必须活过
            // 整个循环体，否则 `stripped`（借用自它）会悬垂。
            let upper = k.to_ascii_uppercase();
            let Some(stripped) = upper.strip_prefix(prefix) else {
                continue;
            };
            if stripped.is_empty() {
                continue;
            }

            // `__` → `.`：figment 的元组 merge 会把点号键组装成嵌套 dict
            let figment_key = stripped.to_lowercase().replace("__", ".");
            // - 值交给 figment 解析（与 `Env` 相同）：`true`/`false` → bool、整数 →
            //   数值、含小数点 → 浮点、其余 → 字符串。
            // - **限制**：纯数字字符串会被推断为数值，因此类型为 `String` 的字段
            //   收不到纯数字配置值（会报类型不匹配）——这是无类型 env 值的固有取舍。
            let value: figment::value::Value = v
                .parse()
                .unwrap_or_else(|_| unreachable!("figment 值解析不失败"));

            figment = figment.merge((figment_key, value));
        }

        let cfg: Self = figment.extract().map_err(ConfigError::Load)?;

        cfg.validate().map_err(|source| ConfigError::Validation {
            prefix,
            source: Box::new(source),
        })?;

        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    // ---- 机制测试用的假配置 ----

    #[derive(Debug, PartialEq, Deserialize)]
    struct SampleConfig {
        #[serde(default)]
        flag: bool,
        #[serde(default)]
        count: u64,
        #[serde(default)]
        ratio: f64,
        #[serde(default)]
        name: String,
        #[serde(default)]
        nested: SampleNested,
    }

    #[derive(Debug, Default, PartialEq, Deserialize)]
    struct SampleNested {
        label: String,
        size: u64,
    }

    impl ConfigMeta for SampleConfig {
        type Error = std::convert::Infallible;
        fn prefix() -> &'static str {
            "SAMPLE_"
        }
        fn validate(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    /// 必填字段，验证"缺失必填 → Load 错误"
    #[derive(Debug, Deserialize)]
    struct StrictConfig {
        required: String,
    }

    impl ConfigMeta for StrictConfig {
        type Error = std::convert::Infallible;
        fn prefix() -> &'static str {
            "STRICT_"
        }
        fn validate(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    /// validate 恒失败，验证"语义非法 → Validation 错误"
    #[derive(Debug, thiserror::Error)]
    #[error("语义校验恒失败")]
    struct AlwaysFails;

    #[derive(Debug, Deserialize)]
    struct RejectConfig {
        #[serde(default)]
        value: u64,
    }

    impl ConfigMeta for RejectConfig {
        type Error = AlwaysFails;
        fn prefix() -> &'static str {
            "REJECT_"
        }
        fn validate(&self) -> Result<(), Self::Error> {
            Err(AlwaysFails)
        }
    }

    // ---- 用例 ----

    fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    /// 前缀剥离 + 平铺键映射 + figment 类型解析（bool/u64/f64/String）
    #[test]
    fn strips_prefix_and_parses_flat_key_value_types() {
        let cfg = SampleConfig::load_from(vars(&[
            ("SAMPLE_FLAG", "false"),
            ("SAMPLE_COUNT", "42"),
            ("SAMPLE_RATIO", "0.5"),
            ("SAMPLE_NAME", "hello"),
        ]))
        .unwrap();

        assert_eq!(cfg.flag, false);
        assert_eq!(cfg.count, 42);
        assert_eq!(cfg.ratio, 0.5);
        assert_eq!(cfg.name, "hello");
        assert_eq!(cfg.nested, SampleNested::default());
    }

    /// 双下划线 = 嵌套层级
    #[test]
    fn double_underscore_maps_to_nested_struct() {
        let cfg = SampleConfig::load_from(vars(&[
            ("SAMPLE_NESTED__LABEL", "inner"),
            ("SAMPLE_NESTED__SIZE", "7"),
        ]))
        .unwrap();
        assert_eq!(
            cfg.nested,
            SampleNested {
                label: "inner".into(),
                size: 7
            }
        );
    }

    /// 单下划线不进嵌套结构（防呆回归：平铺键 `nested_label` 不是字段）
    #[test]
    fn single_underscore_key_does_not_leak_into_nested_struct() {
        let cfg = SampleConfig::load_from(vars(&[("SAMPLE_NESTED_LABEL", "inner")])).unwrap();
        assert_eq!(cfg.nested, SampleNested::default());
    }

    /// 非前缀键、空键被忽略；小写变量名同样可读（大小写不敏感）
    #[test]
    fn non_prefixed_keys_are_ignored_and_case_insensitive() {
        let cfg = SampleConfig::load_from(vars(&[
            ("OTHER_FLAG", "true"),
            ("SAMPLE", ""),
            ("sample_name", "kept"),
        ]))
        .unwrap();

        assert!(!cfg.flag); // 未被 OTHER_FLAG 污染
        assert_eq!(cfg.name, "kept");
    }

    /// 空输入 → serde 默认值
    #[test]
    fn empty_input_falls_back_to_serde_defaults() {
        let cfg = SampleConfig::load_from(Vec::<(String, String)>::new()).unwrap();
        assert!(!cfg.flag);
        assert_eq!(cfg.count, 0);
        assert_eq!(cfg.ratio, 0.0);
        assert_eq!(cfg.name, "");
        assert_eq!(cfg.nested, SampleNested::default());
    }

    /// 必填字段缺失 → ConfigError::Load
    #[test]
    fn missing_required_field_is_load_error() {
        let result = StrictConfig::load_from(Vec::<(String, String)>::new());
        assert!(matches!(result, Err(ConfigError::Load(_))));
    }

    /// 反序列化成功但语义校验失败 → ConfigError::Validation
    #[test]
    fn semantic_validation_failure_is_validation_error() {
        let result = RejectConfig::load_from(vars(&[("REJECT_VALUE", "1")]));
        assert!(matches!(result, Err(ConfigError::Validation { .. })));
    }
}
