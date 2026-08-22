use chrono::{DateTime, Utc};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status};
use uuid::Uuid;

use crate::{
    error::DomainError,
    id::OrganizationId,
    organization::value_object::{OrganizationCode, OrganizationName},
};

/// 组织架构聚合根
#[derive(Debug, Clone)]
pub struct Organization {
    id: OrganizationId,
    parent_id: Option<OrganizationId>,
    name: OrganizationName,
    code: OrganizationCode,
    contact: String,
    phone: String,
    email: String,
    sort: i32,
    remark: Option<String>,
    status: Status,
    audit_meta: AuditMeta,
    delete_meta: DeleteMeta,
}

impl Organization {
    /// 创建新组织
    pub fn new(
        id: OrganizationId,
        parent_id: Option<OrganizationId>,
        name: OrganizationName,
        code: OrganizationCode,
        contact: Option<String>,
        phone: Option<String>,
        email: Option<String>,
        sort: Option<i32>,
        remark: Option<String>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            parent_id,
            name,
            code,
            contact: contact.unwrap_or_default(),
            phone: phone.unwrap_or_default(),
            email: email.unwrap_or_default(),
            sort: sort.unwrap_or(1000),
            remark,
            status: Status::Enabled,
            audit_meta: AuditMeta::new(operator_id, now),
            delete_meta: DeleteMeta::new(),
        }
    }

    /// 从数据库还原
    pub fn restore(
        id: OrganizationId,
        parent_id: Option<OrganizationId>,
        name: OrganizationName,
        code: OrganizationCode,
        contact: String,
        phone: String,
        email: String,
        sort: i32,
        remark: Option<String>,
        status: Status,
        audit_meta: AuditMeta,
        delete_meta: DeleteMeta,
    ) -> Self {
        Self {
            id,
            parent_id,
            name,
            code,
            contact,
            phone,
            email,
            sort,
            remark,
            status,
            audit_meta,
            delete_meta,
        }
    }

    /// 通用修改前置校验：仅拦截已删除的情况
    fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.delete_meta.is_deleted() {
            return Err(DomainError::OrganizationNotFound { id: self.id });
        }
        Ok(())
    }

    /// 更新基本信息（不含 parent_id，移动节点走 move_to）
    pub fn update_info(
        &mut self,
        name: OrganizationName,
        code: OrganizationCode,
        contact: Option<String>,
        phone: Option<String>,
        email: Option<String>,
        remark: Option<String>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        self.name = name;
        self.code = code;
        if let Some(contact) = contact {
            self.contact = contact;
        }
        if let Some(phone) = phone {
            self.phone = phone;
        }
        if let Some(email) = email {
            self.email = email;
        }
        self.remark = remark;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    /// 移动到新的父节点（树形结构调整）
    ///
    /// 只做最基本的"不能把自己设为自己的父节点"校验；
    /// "不能移动到自己的子孙节点下"这类需要遍历树的校验，
    /// 依赖仓储查询子树，超出单个聚合的能力范围，交由应用层在调用前完成
    pub fn move_to(
        &mut self,
        new_parent_id: Option<OrganizationId>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        if new_parent_id == Some(self.id) {
            return Err(DomainError::OrganizationInvalidParent { id: self.id });
        }

        self.parent_id = new_parent_id;
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
            return Err(DomainError::OrganizationStatusAlreadyEnabled { id: self.id });
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
            return Err(DomainError::OrganizationStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        Ok(())
    }

    /// 软删除
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

    /// 删除前置校验：是否还有子组织、是否还有关联用户
    /// has_children / has_members 均由应用层查询后传入，聚合内部不持有这些集合
    pub fn ensure_deletable(
        &self,
        has_children: bool,
        has_members: bool,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if has_children {
            return Err(DomainError::OrganizationHasChildren { id: self.id });
        }
        if has_members {
            return Err(DomainError::OrganizationHasMembers { id: self.id });
        }
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------
    pub fn id(&self) -> OrganizationId {
        self.id
    }
    pub fn parent_id(&self) -> Option<OrganizationId> {
        self.parent_id
    }
    pub fn name(&self) -> &OrganizationName {
        &self.name
    }
    pub fn code(&self) -> &OrganizationCode {
        &self.code
    }
    pub fn contact(&self) -> &str {
        &self.contact
    }
    pub fn phone(&self) -> &str {
        &self.phone
    }
    pub fn email(&self) -> &str {
        &self.email
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
mod organization_aggregate_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap()
    }

    fn operator_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn test_org_id() -> OrganizationId {
        OrganizationId::from_uuid(Uuid::now_v7())
    }

    fn test_name() -> OrganizationName {
        OrganizationName::new("北京分公司").unwrap()
    }

    fn test_code() -> OrganizationCode {
        OrganizationCode::new("beijing").unwrap()
    }

    #[test]
    fn test_new_default_values() {
        let org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            Some(operator_uuid()),
            test_now(),
        );

        assert_eq!(org.parent_id(), None);
        assert_eq!(org.sort(), 1000);
        assert_eq!(org.contact(), "");
        assert_eq!(org.status(), Status::Enabled);
        assert!(!org.delete_meta().is_deleted());
        assert_eq!(org.audit_meta().created_by(), Some(operator_uuid()));
    }

    #[test]
    fn test_update_info_success() {
        let mut org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );

        let new_name = OrganizationName::new("北京总部").unwrap();
        let new_code = OrganizationCode::new("beijing-hq").unwrap();

        org.update_info(
            new_name.clone(),
            new_code.clone(),
            Some("张三".into()),
            Some("13800000000".into()),
            Some("hq@example.com".into()),
            Some("备注".into()),
            Some(operator_uuid()),
            test_now(),
        )
        .unwrap();

        assert_eq!(org.name(), &new_name);
        assert_eq!(org.code(), &new_code);
        assert_eq!(org.contact(), "张三");
        assert_eq!(org.audit_meta().updated_by(), Some(operator_uuid()));
    }

    #[test]
    fn test_update_info_fail_deleted() {
        let mut org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );
        org.delete(Some(operator_uuid()), test_now()).unwrap();

        let err = org
            .update_info(
                test_name(),
                test_code(),
                None,
                None,
                None,
                None,
                Some(operator_uuid()),
                test_now(),
            )
            .unwrap_err();
        assert!(matches!(err, DomainError::OrganizationNotFound { .. }));
    }

    #[test]
    fn test_move_to_success() {
        let mut org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );
        let new_parent = OrganizationId::from_uuid(Uuid::now_v7());

        org.move_to(Some(new_parent), Some(operator_uuid()), test_now())
            .unwrap();
        assert_eq!(org.parent_id(), Some(new_parent));
    }

    #[test]
    fn test_move_to_self_rejected() {
        let id = test_org_id();
        let mut org = Organization::new(
            id,
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );

        let err = org
            .move_to(Some(id), Some(operator_uuid()), test_now())
            .unwrap_err();
        assert!(matches!(err, DomainError::OrganizationInvalidParent { .. }));
    }

    #[test]
    fn test_enable_disable_flow() {
        let mut org = Organization::restore(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            String::new(),
            String::new(),
            String::new(),
            1000,
            None,
            Status::Disabled,
            AuditMeta::new(Some(operator_uuid()), test_now()),
            DeleteMeta::new(),
        );

        org.enable(Some(operator_uuid()), test_now()).unwrap();
        assert_eq!(org.status(), Status::Enabled);

        let err = org.enable(Some(operator_uuid()), test_now()).unwrap_err();
        assert!(matches!(
            err,
            DomainError::OrganizationStatusAlreadyEnabled { .. }
        ));

        org.disable(Some(operator_uuid()), test_now()).unwrap();
        assert_eq!(org.status(), Status::Disabled);
    }

    #[test]
    fn test_ensure_deletable_rejects_with_children() {
        let org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );

        let err = org.ensure_deletable(true, false).unwrap_err();
        assert!(matches!(err, DomainError::OrganizationHasChildren { .. }));
    }

    #[test]
    fn test_ensure_deletable_rejects_with_members() {
        let org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );

        let err = org.ensure_deletable(false, true).unwrap_err();
        assert!(matches!(err, DomainError::OrganizationHasMembers { .. }));
    }

    #[test]
    fn test_ensure_deletable_success() {
        let org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            None,
            test_now(),
        );

        assert!(org.ensure_deletable(false, false).is_ok());
    }

    #[test]
    fn test_soft_delete_already_deleted_fail() {
        let mut org = Organization::new(
            test_org_id(),
            None,
            test_name(),
            test_code(),
            None,
            None,
            None,
            None,
            None,
            Some(operator_uuid()),
            test_now(),
        );

        org.delete(Some(operator_uuid()), test_now()).unwrap();

        let err = org.delete(Some(operator_uuid()), test_now()).unwrap_err();
        assert!(matches!(err, DomainError::OrganizationNotFound { .. }));
    }
}
