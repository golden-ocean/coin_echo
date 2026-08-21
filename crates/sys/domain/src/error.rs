//! sys-domain 顶层错误聚合。

use platform_kernel::error::{ErrorKind, ErrorMeta, FieldError};
use platform_kernel::meta::StatusError;

use crate::dictionary::value_object::{
    DictionaryCodeError, DictionaryItemColorError, DictionaryItemLabelError,
    DictionaryItemValueError, DictionaryNameError,
};
use crate::id::{DictionaryId, DictionaryItemId};

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error(transparent)]
    Status(#[from] StatusError),

    // ---- Dictionary 值对象错误 ----
    #[error(transparent)]
    DictionaryCode(#[from] DictionaryCodeError),
    #[error(transparent)]
    DictionaryName(#[from] DictionaryNameError),

    // ---- Dictionary 状态/权限规则 ----
    #[error("字典 {id} 有字典项")]
    DictionaryHasItems { id: DictionaryId },
    #[error("字典 {id} 已启用")]
    DictionaryStatusAlreadyEnabled { id: DictionaryId },
    #[error("字典 {id} 已禁用")]
    DictionaryStatusAlreadyDisabled { id: DictionaryId },
    #[error("字典 {id} 不存在")]
    DictionaryNotFound { id: DictionaryId },
    #[error("字典 {id} 受系统保护，禁止修改")]
    DictionaryProtected { id: DictionaryId },

    // ---- DictionaryItem 值对象错误 ----
    #[error(transparent)]
    DictionaryItemLabel(#[from] DictionaryItemLabelError),
    #[error(transparent)]
    DictionaryItemValue(#[from] DictionaryItemValueError),
    #[error(transparent)]
    DictionaryItemColor(#[from] DictionaryItemColorError),

    // ---- DictionaryItem 状态/权限规则 ----
    #[error("字典项 {id} 已启用")]
    DictionaryItemStatusAlreadyEnabled { id: DictionaryItemId },
    #[error("字典项 {id} 已禁用")]
    DictionaryItemStatusAlreadyDisabled { id: DictionaryItemId },
    #[error("字典项 {id} 不存在")]
    DictionaryItemNotFound { id: DictionaryItemId },
    #[error("字典项 {id} 受系统保护，禁止修改")]
    DictionaryItemProtected { id: DictionaryItemId },
}

impl ErrorMeta for DomainError {
    fn kind(&self) -> ErrorKind {
        match self {
            // 状态
            Self::Status(e) => e.kind(),

            // Dictionary 值对象错误：委托
            Self::DictionaryCode(e) => e.kind(),
            Self::DictionaryName(e) => e.kind(),

            // Dictionary 状态/权限规则
            Self::DictionaryHasItems { .. } => ErrorKind::Validation,
            Self::DictionaryStatusAlreadyEnabled { .. }
            | Self::DictionaryStatusAlreadyDisabled { .. } => ErrorKind::Conflict,
            Self::DictionaryProtected { .. } => ErrorKind::Forbidden,
            Self::DictionaryNotFound { .. } => ErrorKind::NotFound,

            // DictionaryItem 值对象错误：委托
            Self::DictionaryItemLabel(e) => e.kind(),
            Self::DictionaryItemValue(e) => e.kind(),
            Self::DictionaryItemColor(e) => e.kind(),

            // DictionaryItem 状态/权限规则
            Self::DictionaryItemStatusAlreadyEnabled { .. }
            | Self::DictionaryItemStatusAlreadyDisabled { .. } => ErrorKind::Conflict,
            Self::DictionaryItemProtected { .. } => ErrorKind::Forbidden,
            Self::DictionaryItemNotFound { .. } => ErrorKind::NotFound,
        }
    }

    fn code(&self) -> &'static str {
        match self {
            // 状态
            Self::Status(e) => e.code(),

            // Dictionary
            Self::DictionaryCode(e) => e.code(),
            Self::DictionaryName(e) => e.code(),

            // Dictionary 状态/权限规则
            Self::DictionaryHasItems { .. } => "iam.dictionary.has_items",
            Self::DictionaryStatusAlreadyEnabled { .. } => "iam.dictionary.status.already_enabled",
            Self::DictionaryStatusAlreadyDisabled { .. } => {
                "iam.dictionary.status.already_disabled"
            }
            Self::DictionaryNotFound { .. } => "iam.dictionary.not_found",
            Self::DictionaryProtected { .. } => "iam.dictionary.protected",

            // DictionaryItem
            Self::DictionaryItemLabel(e) => e.code(),
            Self::DictionaryItemValue(e) => e.code(),
            Self::DictionaryItemColor(e) => e.code(),

            Self::DictionaryItemStatusAlreadyEnabled { .. } => {
                "iam.dictionary_item.status.already_enabled"
            }
            Self::DictionaryItemStatusAlreadyDisabled { .. } => {
                "iam.dictionary_item.status.already_disabled"
            }
            Self::DictionaryItemNotFound { .. } => "iam.dictionary_item.not_found",
            Self::DictionaryItemProtected { .. } => "iam.dictionary_item.protected",
        }
    }

    fn detail(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::Status(e) => e.detail(),

            // Dictionary
            Self::DictionaryCode(e) => e.detail(),
            Self::DictionaryName(e) => e.detail(),

            // DictionaryItem
            Self::DictionaryItemLabel(e) => e.detail(),
            Self::DictionaryItemValue(e) => e.detail(),
            Self::DictionaryItemColor(e) => e.detail(),

            _ => None,
        }
    }

    fn fields(&self) -> Vec<FieldError> {
        match self {
            // 状态
            Self::Status(e) => e.fields(),

            // Dictionary VO
            Self::DictionaryCode(e) => e.fields(),
            Self::DictionaryName(e) => e.fields(),

            // DictionaryItem
            Self::DictionaryItemLabel(e) => e.fields(),
            Self::DictionaryItemValue(e) => e.fields(),
            Self::DictionaryItemColor(e) => e.fields(),

            _ => Vec::new(),
        }
    }
}
