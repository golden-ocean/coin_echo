use chrono::{DateTime, Utc};
use platform_kernel::meta::{AuditMeta, Status};
use uuid::Uuid;

use crate::{
    dictionary::value_object::{DictionaryItemColor, DictionaryItemLabel, DictionaryItemValue},
    error::DomainError,
    id::{DictionaryId, DictionaryItemId},
};

#[derive(Debug, Clone)]
pub struct DictionaryItem {
    id: DictionaryItemId,
    dictionary_id: DictionaryId,
    label: DictionaryItemLabel,
    value: DictionaryItemValue,
    color: Option<DictionaryItemColor>,
    is_builtin: bool,

    sort: i32,
    remark: Option<String>,
    status: Status,
    audit_meta: AuditMeta,
}

impl DictionaryItem {
    pub fn new(
        id: DictionaryItemId,
        dictionary_id: DictionaryId,
        label: DictionaryItemLabel,
        value: DictionaryItemValue,
        color: Option<DictionaryItemColor>,
        operator_id: Option<Uuid>,
        sort: Option<i32>,
        remark: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            dictionary_id,
            label,
            value,
            color,
            is_builtin: false,
            sort: sort.unwrap_or(1000),
            remark,
            status: Status::Enabled,
            audit_meta: AuditMeta::new(operator_id, now),
        }
    }

    pub fn restore(
        id: DictionaryItemId,
        dictionary_id: DictionaryId,
        label: DictionaryItemLabel,
        value: DictionaryItemValue,
        color: Option<DictionaryItemColor>,
        is_builtin: bool,
        sort: i32,
        remark: Option<String>,
        status: Status,
        audit_meta: AuditMeta,
    ) -> Self {
        Self {
            id,
            dictionary_id,
            label,
            value,
            color,
            is_builtin,
            sort,
            remark,
            status,
            audit_meta,
        }
    }

    /// 内置项保护
    pub fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.is_builtin {
            return Err(DomainError::DictionaryItemProtected { id: self.id });
        }
        Ok(())
    }

    pub fn update_info(
        &mut self,
        label: DictionaryItemLabel,
        color: Option<DictionaryItemColor>,
        remark: Option<String>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        self.label = label;
        self.color = color;
        self.remark = remark;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    pub fn disable(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status.is_disabled() {
            return Err(DomainError::DictionaryItemStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    pub fn enable(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status.is_enabled() {
            return Err(DomainError::DictionaryItemStatusAlreadyEnabled { id: self.id });
        }
        self.status = Status::Enabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    pub fn ensure_deletable(&self) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        Ok(())
    }

    // ---- getters ----
    pub fn id(&self) -> DictionaryItemId {
        self.id
    }
    pub fn dictionary_id(&self) -> DictionaryId {
        self.dictionary_id
    }

    pub fn label(&self) -> &DictionaryItemLabel {
        &self.label
    }
    pub fn value(&self) -> &DictionaryItemValue {
        &self.value
    }
    pub fn color(&self) -> Option<&DictionaryItemColor> {
        self.color.as_ref()
    }
    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }
    pub fn sort(&self) -> i32 {
        self.sort
    }
    pub fn remark(&self) -> Option<&str> {
        self.remark.as_deref()
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn audit_meta(&self) -> &AuditMeta {
        &self.audit_meta
    }
}
