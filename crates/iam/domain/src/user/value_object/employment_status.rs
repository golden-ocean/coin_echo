use std::fmt;
use std::str::FromStr;

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmploymentStatusError {
    #[error("工作状态不能为空")]
    Empty,
    #[error("工作状态格式无效: {value}, 合法值：active / on_leave / resigned / terminated")]
    Invalid { value: String },
}

impl ErrorMeta for EmploymentStatusError {
    fn kind(&self) -> ErrorKind {
        ErrorKind::Validation
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Empty => "iam.user.employment_status.empty",
            Self::Invalid { .. } => "iam.user.employment_status.invalid",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Invalid { value } => Some(
                format!(
                    "工作状态格式无效: {value}, 合法值：active / on_leave / resigned / terminated"
                )
                .into(),
            ),
            Self::Empty => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        let code = match self {
            Self::Empty => "required",
            Self::Invalid { .. } => "invalid_enum_value",
        };
        vec![FieldError::new("employment_status", code)]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum EmploymentStatus {
    #[default]
    Active,
    OnLeave,
    Resigned,
    Terminated,
}

impl EmploymentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::OnLeave => "on_leave",
            Self::Resigned => "resigned",
            Self::Terminated => "terminated",
        }
    }

    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }
    pub fn is_on_leave(&self) -> bool {
        matches!(self, Self::OnLeave)
    }
    pub fn is_resigned(&self) -> bool {
        matches!(self, Self::Resigned)
    }
    pub fn is_terminated(&self) -> bool {
        matches!(self, Self::Terminated)
    }
    pub fn is_left_company(&self) -> bool {
        matches!(self, Self::Resigned | Self::Terminated)
    }
    pub fn is_still_employed(&self) -> bool {
        matches!(self, Self::Active | Self::OnLeave)
    }
}

impl fmt::Display for EmploymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for EmploymentStatus {
    type Err = EmploymentStatusError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let raw = s.trim().to_ascii_lowercase();
        if raw.is_empty() {
            return Err(EmploymentStatusError::Empty);
        }
        match raw.as_str() {
            "active" => Ok(Self::Active),
            "on_leave" => Ok(Self::OnLeave),
            "resigned" => Ok(Self::Resigned),
            "terminated" => Ok(Self::Terminated),
            _ => Err(EmploymentStatusError::Invalid { value: raw }),
        }
    }
}

impl AsRef<str> for EmploymentStatus {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<&str> for EmploymentStatus {
    type Error = EmploymentStatusError;

    #[inline]
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value)
    }
}

impl TryFrom<String> for EmploymentStatus {
    type Error = EmploymentStatusError;

    #[inline]
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value)
    }
}

impl TryFrom<&String> for EmploymentStatus {
    type Error = EmploymentStatusError;

    #[inline]
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::from_str(value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_status() {
        let status = EmploymentStatus::default();
        assert_eq!(status, EmploymentStatus::Active);
        assert!(status.is_active());
        assert!(status.is_still_employed());
        assert!(!status.is_left_company());
    }

    #[test]
    fn test_from_str_valid_cases() {
        let cases = [
            ("active", EmploymentStatus::Active),
            ("ACTIVE", EmploymentStatus::Active),
            ("  On_Leave  ", EmploymentStatus::OnLeave),
            ("RESIGNED", EmploymentStatus::Resigned),
            ("terminated", EmploymentStatus::Terminated),
        ];
        for (value, expect) in cases {
            let parsed = EmploymentStatus::from_str(value).unwrap();
            assert_eq!(parsed, expect);
            let parsed_parse = value.parse::<EmploymentStatus>().unwrap();
            assert_eq!(parsed_parse, expect);
        }
    }

    #[test]
    fn test_from_str_empty_error() {
        let err1 = EmploymentStatus::from_str("").unwrap_err();
        assert_eq!(err1, EmploymentStatusError::Empty);
        let err2 = EmploymentStatus::from_str("    ").unwrap_err();
        assert_eq!(err2, EmploymentStatusError::Empty);
    }

    #[test]
    fn test_from_str_invalid_error() {
        let bad_inputs = ["leave", "quit", "fire", "abc123", "off_work"];
        for s in bad_inputs {
            let err = EmploymentStatus::from_str(s).unwrap_err();
            assert_eq!(
                err,
                EmploymentStatusError::Invalid {
                    value: s.to_string()
                }
            );
            assert!(err.to_string().contains("工作状态格式无效"));
            assert!(err.to_string().contains(s));
        }
    }

    #[test]
    fn test_try_from_str_and_string() {
        // &str
        let status_ref: EmploymentStatus = "on_leave".try_into().unwrap();
        assert_eq!(status_ref, EmploymentStatus::OnLeave);

        // String
        let status_string: EmploymentStatus = String::from("RESIGNED").try_into().unwrap();
        assert_eq!(status_string, EmploymentStatus::Resigned);

        // &String
        let s = String::from("terminated");
        let status_ref_string: EmploymentStatus = (&s).try_into().unwrap();
        assert_eq!(status_ref_string, EmploymentStatus::Terminated);

        // Error path via TryFrom
        let err: Result<EmploymentStatus, _> = "retired".try_into();
        assert_eq!(
            err.unwrap_err(),
            EmploymentStatusError::Invalid {
                value: "retired".to_string()
            }
        );
    }

    #[test]
    fn test_string_output_consistent() {
        let cases = [
            (EmploymentStatus::Active, "active"),
            (EmploymentStatus::OnLeave, "on_leave"),
            (EmploymentStatus::Resigned, "resigned"),
            (EmploymentStatus::Terminated, "terminated"),
        ];
        for (status, expect) in cases {
            assert_eq!(status.as_str(), expect);
            assert_eq!(status.to_string(), expect);
            assert_eq!(status.as_ref(), expect);
        }
    }

    #[test]
    fn test_bool_judge_methods() {
        let s_active = EmploymentStatus::Active;
        assert!(s_active.is_active());
        assert!(s_active.is_still_employed());
        assert!(!s_active.is_left_company());

        let s_leave = EmploymentStatus::OnLeave;
        assert!(s_leave.is_on_leave());
        assert!(s_leave.is_still_employed());
        assert!(!s_leave.is_left_company());

        let s_resign = EmploymentStatus::Resigned;
        assert!(s_resign.is_resigned());
        assert!(s_resign.is_left_company());
        assert!(!s_resign.is_still_employed());

        let s_term = EmploymentStatus::Terminated;
        assert!(s_term.is_terminated());
        assert!(s_term.is_left_company());
        assert!(!s_term.is_still_employed());
    }

    // ---- ErrorMeta ----

    #[test]
    fn error_meta_kind_is_always_validation() {
        assert_eq!(EmploymentStatusError::Empty.kind(), ErrorKind::Validation);
        assert_eq!(
            EmploymentStatusError::Invalid { value: "x".into() }.kind(),
            ErrorKind::Validation
        );
    }

    #[test]
    fn error_meta_codes_are_distinct_and_namespaced() {
        let empty = EmploymentStatusError::Empty.code();
        let invalid = EmploymentStatusError::Invalid { value: "x".into() }.code();
        assert_ne!(empty, invalid);
        assert!(empty.starts_with("iam.user.employment_status."));
    }

    #[test]
    fn error_meta_fields_names_employment_status_field() {
        let fields = EmploymentStatusError::Empty.fields();
        assert_eq!(fields[0].field, "employment_status");
        assert_eq!(fields[0].code, "required");
    }
}
