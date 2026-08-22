use chrono::{DateTime, Utc};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status};
use uuid::Uuid;

use crate::{
    error::DomainError,
    id::PositionId,
    position::value_object::{PositionCode, PositionName},
};

/// 职位聚合根（全局定义，不隶属具体组织；
/// 用户在哪个组织担任哪个职位由 iam_user.organization_id / position_id 表达）
#[derive(Debug, Clone)]
pub struct Position {
    id: PositionId,
    name: PositionName,
    code: PositionCode,
    sort: i32,
    remark: Option<String>,
    status: Status,
    audit_meta: AuditMeta,
    delete_meta: DeleteMeta,
}

impl Position {
    pub fn new(
        id: PositionId,
        name: PositionName,
        code: PositionCode,
        sort: Option<i32>,
        remark: Option<String>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            code,
            sort: sort.unwrap_or(1000),
            remark,
            status: Status::Enabled,
            audit_meta: AuditMeta::new(operator_id, now),
            delete_meta: DeleteMeta::new(),
        }
    }

    pub fn restore(
        id: PositionId,
        name: PositionName,
        code: PositionCode,
        sort: i32,
        remark: Option<String>,
        status: Status,
        audit_meta: AuditMeta,
        delete_meta: DeleteMeta,
    ) -> Self {
        Self {
            id,
            name,
            code,
            sort,
            remark,
            status,
            audit_meta,
            delete_meta,
        }
    }

    fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.delete_meta.is_deleted() {
            return Err(DomainError::PositionNotFound { id: self.id });
        }
        Ok(())
    }

    pub fn update_info(
        &mut self,
        name: PositionName,
        code: PositionCode,
        remark: Option<String>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        self.name = name;
        self.code = code;
        self.remark = remark;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    pub fn reorder(&mut self, sort: i32, operator_id: Option<Uuid>, now: DateTime<Utc>) {
        self.sort = sort;
        self.audit_meta = self.audit_meta.update(operator_id, now);
    }

    pub fn enable(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status.is_enabled() {
            return Err(DomainError::PositionStatusAlreadyEnabled { id: self.id });
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
            return Err(DomainError::PositionStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    pub fn delete(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        self.delete_meta = self.delete_meta.delete(operator_id, now);
        Ok(())
    }

    /// 删除前置校验：是否还有用户正担任该职位
    /// has_members 由应用层查询 iam_user（WHERE position_id = ?）后传入
    pub fn ensure_deletable(&self, has_members: bool) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if has_members {
            return Err(DomainError::PositionHasMembers { id: self.id });
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------
    pub fn id(&self) -> PositionId {
        self.id
    }
    pub fn name(&self) -> &PositionName {
        &self.name
    }
    pub fn code(&self) -> &PositionCode {
        &self.code
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
    pub fn delete_meta(&self) -> &DeleteMeta {
        &self.delete_meta
    }
}

// ========================= 单元测试模块 =========================
#[cfg(test)]
mod position_aggregate_tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap()
    }

    fn operator_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn test_position_id() -> PositionId {
        PositionId::from_uuid(Uuid::now_v7())
    }

    fn test_name() -> PositionName {
        PositionName::new("经理").unwrap()
    }

    fn test_code() -> PositionCode {
        PositionCode::new("manager").unwrap()
    }

    #[test]
    fn test_new_default_values() {
        let pos = Position::new(
            test_position_id(),
            test_name(),
            test_code(),
            None,
            None,
            Some(operator_uuid()),
            test_now(),
        );

        assert_eq!(pos.sort(), 1000);
        assert_eq!(pos.status(), Status::Enabled);
        assert!(!pos.delete_meta().is_deleted());
    }

    #[test]
    fn test_update_info_success() {
        let mut pos = Position::new(
            test_position_id(),
            test_name(),
            test_code(),
            None,
            None,
            None,
            test_now(),
        );
        let new_name = PositionName::new("高级经理").unwrap();
        let new_code = PositionCode::new("senior_manager").unwrap();

        pos.update_info(
            new_name.clone(),
            new_code.clone(),
            Some("备注".into()),
            Some(operator_uuid()),
            test_now(),
        )
        .unwrap();

        assert_eq!(pos.name(), &new_name);
        assert_eq!(pos.code(), &new_code);
    }

    #[test]
    fn test_enable_disable_flow() {
        let mut pos = Position::restore(
            test_position_id(),
            test_name(),
            test_code(),
            1000,
            None,
            Status::Disabled,
            AuditMeta::new(Some(operator_uuid()), test_now()),
            DeleteMeta::new(),
        );

        pos.enable(Some(operator_uuid()), test_now()).unwrap();
        assert_eq!(pos.status(), Status::Enabled);

        let err = pos.enable(Some(operator_uuid()), test_now()).unwrap_err();
        assert!(matches!(
            err,
            DomainError::PositionStatusAlreadyEnabled { .. }
        ));
    }

    #[test]
    fn test_ensure_deletable_rejects_with_members() {
        let pos = Position::new(
            test_position_id(),
            test_name(),
            test_code(),
            None,
            None,
            None,
            test_now(),
        );
        let err = pos.ensure_deletable(true).unwrap_err();
        assert!(matches!(err, DomainError::PositionHasMembers { .. }));
    }

    #[test]
    fn test_soft_delete_already_deleted_fail() {
        let mut pos = Position::new(
            test_position_id(),
            test_name(),
            test_code(),
            None,
            None,
            Some(operator_uuid()),
            test_now(),
        );
        pos.delete(Some(operator_uuid()), test_now()).unwrap();

        let err = pos.delete(Some(operator_uuid()), test_now()).unwrap_err();
        assert!(matches!(err, DomainError::PositionNotFound { .. }));
    }
}
