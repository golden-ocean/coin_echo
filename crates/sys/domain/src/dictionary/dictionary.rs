use chrono::{DateTime, Utc};
use uuid::Uuid;

use platform_kernel::meta::{AuditMeta, Status};

use crate::{
    dictionary::value_object::{DictionaryCode, DictionaryName},
    error::DomainError,
    id::DictionaryId,
};

/// 字典聚合根
#[derive(Debug)]
pub struct Dictionary {
    id: DictionaryId,
    name: DictionaryName,
    code: DictionaryCode,
    is_builtin: bool,

    sort: i32,
    remark: Option<String>,
    status: Status,
    audit_meta: AuditMeta,
}

impl Dictionary {
    pub fn new(
        id: DictionaryId,
        name: DictionaryName,
        code: DictionaryCode,
        operator_id: Option<Uuid>,
        sort: Option<i32>,
        remark: Option<String>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            code,
            is_builtin: false,
            sort: sort.unwrap_or(1000),
            remark,
            status: Status::Enabled,
            audit_meta: AuditMeta::new(operator_id, now),
        }
    }

    pub fn restore(
        id: DictionaryId,
        name: DictionaryName,
        code: DictionaryCode,
        is_builtin: bool,
        sort: i32,
        remark: Option<String>,
        status: Status,
        audit_meta: AuditMeta,
    ) -> Self {
        Self {
            id,
            name,
            code,
            is_builtin,
            sort,
            remark,
            status,
            audit_meta,
        }
    }

    /// 通用修改前置校验：内置账号
    pub fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.is_builtin {
            return Err(DomainError::DictionaryProtected { id: self.id });
        }
        Ok(())
    }

    // 业务方法
    pub fn update_info(
        &mut self,
        name: DictionaryName,
        remark: Option<String>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        self.name = name;
        self.remark = remark;
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
            return Err(DomainError::DictionaryStatusAlreadyEnabled { id: self.id });
        }
        self.status = Status::Enabled;
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
            return Err(DomainError::DictionaryStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    pub fn ensure_deletable(&self, has_items: bool) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if has_items {
            return Err(DomainError::DictionaryHasItems { id: self.id });
        }
        Ok(())
    }

    // ===================== 字段只读Getter =====================
    pub fn id(&self) -> &DictionaryId {
        &self.id
    }
    pub fn name(&self) -> &DictionaryName {
        &self.name
    }
    pub fn code(&self) -> &DictionaryCode {
        &self.code
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
