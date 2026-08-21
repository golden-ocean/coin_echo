use chrono::{DateTime, Utc};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status, VersionMeta};
use uuid::Uuid;

use crate::{
    error::DomainError,
    id::{PermissionId, RoleId},
    role::value_object::{RoleCode, RoleName},
};

#[derive(Debug, Clone)]
pub struct Role {
    id: RoleId,
    name: RoleName,
    code: RoleCode,
    is_builtin: bool,
    remark: Option<String>,
    sort: i32,
    status: Status,
    // 关联绑定的权限列表
    permission_ids: Vec<PermissionId>,
    // 审计与并发控制属性
    audit_meta: AuditMeta,
    delete_meta: DeleteMeta,
    version_meta: VersionMeta,
}

impl Role {
    /// 创建新角色
    pub fn new(
        id: RoleId,
        name: RoleName,
        code: RoleCode,
        remark: Option<String>,
        sort: Option<i32>,
        status: Option<Status>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            code,
            is_builtin: false,
            remark,
            sort: sort.unwrap_or(1000),
            status: status.unwrap_or(Status::Enabled),
            permission_ids: vec![],
            audit_meta: AuditMeta::new(operator_id, now),
            delete_meta: DeleteMeta::new(),
            version_meta: VersionMeta::new(),
        }
    }

    /// 从数据库还原
    pub fn restore(
        id: RoleId,
        name: RoleName,
        code: RoleCode,
        is_builtin: bool,
        remark: Option<String>,
        sort: i32,
        status: Status,
        permission_ids: Vec<PermissionId>,
        audit_meta: AuditMeta,
        delete_meta: DeleteMeta,
        version_meta: VersionMeta,
    ) -> Self {
        Self {
            id,
            name,
            code,
            is_builtin,
            remark,
            sort,
            status,
            permission_ids,
            audit_meta,
            delete_meta,
            version_meta,
        }
    }

    /// 通用修改前置校验：内置、已删除拦截
    fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.is_builtin {
            return Err(DomainError::RoleProtected { id: self.id });
        }
        if self.delete_meta.is_deleted() {
            return Err(DomainError::RoleNotFound { id: self.id });
        }
        Ok(())
    }

    /// 更新角色基本信息
    pub fn update_info(
        &mut self,
        new_name: RoleName,
        new_code: RoleCode,
        new_remark: Option<String>,
        new_sort: Option<i32>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        self.name = new_name;
        self.code = new_code;
        if let Some(remark) = new_remark {
            self.remark = Some(remark);
        }
        if let Some(sort) = new_sort {
            self.sort = sort;
        }
        self.audit_meta = self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 删除角色
    pub fn delete(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        self.audit_meta = self.audit_meta.update(operator_id, now);
        self.delete_meta = self.delete_meta.delete(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 启用角色
    pub fn enable(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status.is_enabled() {
            return Err(DomainError::RoleStatusAlreadyEnabled { id: self.id });
        }
        self.status = Status::Enabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 禁用角色
    pub fn disable(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status.is_disabled() {
            return Err(DomainError::RoleStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 重新为角色分配权限列表
    pub fn assign_permissions(
        &mut self,
        permission_ids: Vec<PermissionId>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        self.permission_ids = permission_ids;
        self.audit_meta = self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------
    pub fn id(&self) -> RoleId {
        self.id
    }
    pub fn name(&self) -> &RoleName {
        &self.name
    }
    pub fn code(&self) -> &RoleCode {
        &self.code
    }
    pub fn is_builtin(&self) -> bool {
        self.is_builtin
    }
    pub fn remark(&self) -> Option<&str> {
        self.remark.as_deref()
    }
    pub fn sort(&self) -> i32 {
        self.sort
    }
    pub fn status(&self) -> Status {
        self.status
    }
    pub fn permission_ids(&self) -> &[PermissionId] {
        &self.permission_ids
    }
    pub fn audit_meta(&self) -> &AuditMeta {
        &self.audit_meta
    }
    pub fn delete_meta(&self) -> &DeleteMeta {
        &self.delete_meta
    }
    pub fn version_meta(&self) -> &VersionMeta {
        &self.version_meta
    }
}

// ========================= 单元测试模块 =========================
#[cfg(test)]
mod role_aggregate_tests {
    use super::*;
    use chrono::{DateTime, TimeZone, Utc};
    use platform_kernel::meta::Status;
    use uuid::Uuid;

    use crate::{
        error::DomainError,
        id::{PermissionId, RoleId},
        role::value_object::{RoleCode, RoleName},
    };

    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap()
    }

    fn operator_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn operator_user_id() -> Uuid {
        operator_uuid()
    }

    fn test_role_id() -> RoleId {
        RoleId::from_uuid(Uuid::now_v7())
    }

    fn test_role_name() -> RoleName {
        RoleName::new("normal_role").unwrap()
    }

    fn test_role_code() -> RoleCode {
        RoleCode::new("NORMAL_ROLE").unwrap()
    }

    fn builtin_role_name() -> RoleName {
        RoleName::new("super_admin").unwrap()
    }

    fn builtin_role_code() -> RoleCode {
        RoleCode::new("SUPER_ADMIN").unwrap()
    }

    /// 测试：new 创建角色默认参数正确性
    #[test]
    fn test_role_new_default_values() {
        let rid = test_role_id();
        let role = Role::new(
            rid,
            test_role_name(),
            test_role_code(),
            None,
            None,
            None,
            Some(operator_user_id()),
            test_now(),
        );

        assert!(!role.is_builtin());
        assert_eq!(role.sort(), 1000);
        assert_eq!(role.status(), Status::Enabled);
        assert!(role.permission_ids().is_empty());
        assert!(!role.delete_meta().is_deleted());
        assert_eq!(role.version_meta().value(), 0);
        assert_eq!(role.audit_meta().created_by(), Some(operator_uuid()));
    }

    /// 测试：update_info 正常修改基础信息
    #[test]
    fn test_update_info_success() {
        let mut role = Role::new(
            test_role_id(),
            test_role_name(),
            test_role_code(),
            Some("old remark".to_string()),
            Some(500),
            Some(Status::Enabled),
            None,
            test_now(),
        );
        let new_name = RoleName::new("new_name").unwrap();
        let new_code = RoleCode::new("NEW_CODE").unwrap();

        role.update_info(
            new_name.clone(),
            new_code.clone(),
            Some("new remark".into()),
            Some(600),
            Some(operator_user_id()),
            test_now(),
        )
        .unwrap();

        assert_eq!(role.name(), &new_name);
        assert_eq!(role.code(), &new_code);
        assert_eq!(role.remark(), Some("new remark"));
        assert_eq!(role.sort(), 600);
        assert_eq!(role.version_meta().value(), 1);
        assert_eq!(role.audit_meta().updated_by(), Some(operator_uuid()));
    }

    /// 测试：内置角色禁止执行 update_info
    #[test]
    fn test_update_info_fail_builtin() {
        let mut builtin_role = Role::restore(
            test_role_id(),
            builtin_role_name(),
            builtin_role_code(),
            true,
            None,
            100,
            Status::Enabled,
            vec![],
            platform_kernel::meta::AuditMeta::new(Some(operator_uuid()), test_now()),
            platform_kernel::meta::DeleteMeta::new(),
            platform_kernel::meta::VersionMeta::new(),
        );

        let err = builtin_role
            .update_info(
                builtin_role_name(),
                builtin_role_code(),
                None,
                Some(99),
                Some(operator_user_id()),
                test_now(),
            )
            .unwrap_err();

        assert!(matches!(err, DomainError::RoleProtected { .. }));
    }

    /// 测试：已删除角色禁止修改信息
    #[test]
    fn test_update_info_fail_deleted() {
        let mut role = Role::new(
            test_role_id(),
            test_role_name(),
            test_role_code(),
            None,
            None,
            None,
            None,
            test_now(),
        );
        // 先软删除
        role.delete(Some(operator_user_id()), test_now()).unwrap();

        let err = role
            .update_info(
                test_role_name(),
                test_role_code(),
                None,
                Some(100),
                Some(operator_user_id()),
                test_now(),
            )
            .unwrap_err();
        assert!(matches!(err, DomainError::RoleNotFound { .. }));
    }

    /// 测试：启用、禁用状态流转
    #[test]
    fn test_enable_disable_flow() {
        let mut role = Role::new(
            test_role_id(),
            test_role_name(),
            test_role_code(),
            None,
            None,
            Some(Status::Disabled),
            None,
            test_now(),
        );

        // 启用
        role.enable(Some(operator_uuid()), test_now()).unwrap();
        assert_eq!(role.status(), Status::Enabled);

        // 重复启用报错
        let err = role.enable(Some(operator_uuid()), test_now()).unwrap_err();
        assert!(matches!(err, DomainError::RoleStatusAlreadyEnabled { .. }));

        // 禁用
        role.disable(Some(operator_uuid()), test_now()).unwrap();
        assert_eq!(role.status(), Status::Disabled);

        // 重复禁用报错
        let err = role.disable(Some(operator_uuid()), test_now()).unwrap_err();
        assert!(matches!(err, DomainError::RoleStatusAlreadyDisabled { .. }));
    }

    /// 测试：内置角色禁止软删除
    #[test]
    fn test_soft_delete_builtin_reject() {
        let mut builtin_role = Role::restore(
            test_role_id(),
            builtin_role_name(),
            builtin_role_code(),
            true,
            None,
            100,
            Status::Enabled,
            vec![],
            platform_kernel::meta::AuditMeta::new(Some(operator_uuid()), test_now()),
            platform_kernel::meta::DeleteMeta::new(),
            platform_kernel::meta::VersionMeta::new(),
        );

        let err = builtin_role
            .delete(Some(operator_user_id()), test_now())
            .unwrap_err();
        assert!(matches!(err, DomainError::RoleProtected { .. }));
        assert!(!builtin_role.delete_meta().is_deleted());
    }

    /// 测试：普通角色软删除成功
    #[test]
    fn test_soft_delete_normal_role() {
        let mut role = Role::new(
            test_role_id(),
            test_role_name(),
            test_role_code(),
            None,
            None,
            None,
            None,
            test_now(),
        );
        role.delete(Some(operator_user_id()), test_now()).unwrap();
        assert!(role.delete_meta().is_deleted());
        assert_eq!(role.audit_meta().updated_by(), Some(operator_uuid()));
    }

    /// 测试：权限批量分配功能
    #[test]
    fn test_assign_permissions_override() {
        let mut role = Role::new(
            test_role_id(),
            test_role_name(),
            test_role_code(),
            None,
            None,
            None,
            None,
            test_now(),
        );
        let pid1 = PermissionId::from_uuid(Uuid::now_v7());
        let pid2 = PermissionId::from_uuid(Uuid::now_v7());

        let _ = role.assign_permissions(vec![pid1, pid2], Some(operator_uuid()), test_now());
        assert_eq!(role.permission_ids().len(), 2);

        // 覆盖为空集合
        let _ = role.assign_permissions(vec![], Some(operator_uuid()), test_now());
        assert!(role.permission_ids().is_empty());
    }

    /// 测试 restore 构造聚合
    #[test]
    fn test_restore_aggregate() {
        let rid = test_role_id();
        let pid = PermissionId::from_uuid(Uuid::now_v7());
        let role = Role::restore(
            rid,
            test_role_name(),
            test_role_code(),
            false,
            Some("restore test".into()),
            200,
            Status::Disabled,
            vec![pid],
            platform_kernel::meta::AuditMeta::new(Some(operator_uuid()), test_now()),
            platform_kernel::meta::DeleteMeta::new(),
            platform_kernel::meta::VersionMeta::new(),
        );

        assert_eq!(role.id(), rid);
        assert_eq!(role.sort(), 200);
        assert_eq!(role.status(), Status::Disabled);
        assert_eq!(role.permission_ids().len(), 1);
        assert_eq!(role.audit_meta().created_by(), Some(operator_uuid()));
    }

    #[test]
    fn test_soft_delete_already_deleted_fail() {
        let mut role = Role::new(
            test_role_id(),
            test_role_name(),
            test_role_code(),
            None,
            None,
            None,
            Some(operator_user_id()),
            test_now(),
        );

        // 第一次软删除成功
        role.delete(Some(operator_user_id()), test_now()).unwrap();

        // 第二次软删除拦截，返回 RoleNotFound
        let err = role
            .delete(Some(operator_user_id()), test_now())
            .unwrap_err();
        assert!(matches!(err, DomainError::RoleNotFound { .. }));
    }
}
