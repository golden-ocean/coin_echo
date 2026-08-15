use std::{fmt, str::FromStr};

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

/// HTTP 请求方法校验错误枚举
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ApiMethodError {
    #[error("不支持的 HTTP 方法: {value}")]
    Unsupported { value: String },
}

impl ErrorMeta for ApiMethodError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "iam.permission.api_method.unsupported",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Unsupported { value } => Some(format!("不支持的 HTTP 方法: '{value}'").into()),
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        vec![FieldError::new("api_method", "invalid_enum")]
    }
}

/// 后端接口请求方法 值对象 VO（仅 kind = Api 时使用）
///
/// # 创建方式
/// - ApiMethod::from_str(&str) -> Result<Self, ApiMethodError>（大小写不敏感）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl ApiMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

impl fmt::Display for ApiMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ApiMethod {
    type Err = ApiMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "PUT" => Ok(Self::Put),
            "DELETE" => Ok(Self::Delete),
            "PATCH" => Ok(Self::Patch),
            "HEAD" => Ok(Self::Head),
            "OPTIONS" => Ok(Self::Options),
            other => Err(ApiMethodError::Unsupported {
                value: other.to_string(),
            }),
        }
    }
}

impl TryFrom<&str> for ApiMethod {
    type Error = ApiMethodError;

    #[inline]
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Self::from_str(s)
    }
}

impl TryFrom<String> for ApiMethod {
    type Error = ApiMethodError;

    #[inline]
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_str(&s)
    }
}

impl AsRef<str> for ApiMethod {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod api_method_tests {
    use super::*;
    use platform_kernel::error::{ErrorKind, ErrorMeta};
    use std::str::FromStr;

    #[test]
    fn test_api_method_valid_cases() {
        assert_eq!(ApiMethod::from_str("GET").unwrap(), ApiMethod::Get);
        assert_eq!(ApiMethod::from_str("post").unwrap(), ApiMethod::Post);
        assert_eq!(ApiMethod::from_str("Delete").unwrap(), ApiMethod::Delete);

        assert_eq!(ApiMethod::Get.as_str(), "GET");
        assert_eq!(ApiMethod::Get.to_string(), "GET");
    }

    #[test]
    fn test_api_method_try_from() {
        let method1: ApiMethod = "put".try_into().unwrap();
        assert_eq!(method1, ApiMethod::Put);

        let raw_string = String::from("PATCH");
        let method2: ApiMethod = raw_string.try_into().unwrap();
        assert_eq!(method2, ApiMethod::Patch);

        let err = ApiMethod::try_from("TRACE").unwrap_err();
        assert_eq!(
            err,
            ApiMethodError::Unsupported {
                value: "TRACE".to_string()
            }
        );
    }

    #[test]
    fn test_api_method_invalid_cases() {
        assert_eq!(
            ApiMethod::from_str(""),
            Err(ApiMethodError::Unsupported {
                value: "".to_string()
            })
        );
        assert_eq!(
            ApiMethod::from_str("CONNECT"),
            Err(ApiMethodError::Unsupported {
                value: "CONNECT".to_string()
            })
        );
    }

    #[test]
    fn test_api_method_error_meta() {
        let err = ApiMethodError::Unsupported {
            value: "TRACE".to_string(),
        };
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert_eq!(err.code(), "iam.permission.api_method.unsupported");
        assert_eq!(err.fields()[0].field, "api_method");
        assert_eq!(err.detail().unwrap(), "不支持的 HTTP 方法: 'TRACE'");
    }
}
