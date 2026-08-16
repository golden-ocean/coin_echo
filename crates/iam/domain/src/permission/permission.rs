use chrono::{DateTime, Utc};
use uuid::Uuid;

use platform_kernel::meta::{AuditMeta, DeleteMeta, Status, VersionMeta};

use crate::{
    error::DomainError,
    id::PermissionId,
    permission::value_object::{ApiMethod, PermissionCode, PermissionKind, PermissionName},
};

/// 权限/菜单领域聚合根
/// 业务约束：
/// 1. 内置权限 is_builtin = true 禁止修改、禁用、删除，受系统保护（防止超管误删基础菜单）
/// 2. 已软删除权限无法执行任何修改操作
/// 3. 权限类型（kind）与附属字段存在弱约束：
///    - Menu: 建议填写 route_path / component / icon
///    - Api:  建议填写 api_method / api_path
///    - Button: 通常不需要以上任何附属字段
///    这里只做"类型内部字段一致性"的浅校验，不强制业务方必须填写，
///    仅拦截"明显不匹配"的组合（如 Button 类型却填了 api_path）。
/// 4. parent_id 的多级循环引用检测（A→B→C→A）无法在聚合根内完成，
///    因为需要遍历完整权限树，必须由仓储层在 change_parent 前查询校验。
///    聚合根内只拦截最直接的一种情况：把自己设为自己的父级。
#[derive(Debug, Clone)]
pub struct Permission {
    id: PermissionId,
    parent_id: Option<PermissionId>,
    name: PermissionName,
    code: PermissionCode,
    kind: PermissionKind,

    // 菜单专属字段
    route_path: Option<String>,
    component: Option<String>,
    icon: Option<String>,

    // 接口专属字段
    api_method: Option<ApiMethod>,
    api_path: Option<String>,

    is_builtin: bool,
    remark: Option<String>,
    sort: i32,
    status: Status,

    audit_meta: AuditMeta,
    delete_meta: DeleteMeta,
    version_meta: VersionMeta,
}

impl Permission {
    /// 创建新权限
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: PermissionId,
        parent_id: Option<PermissionId>,
        name: PermissionName,
        code: PermissionCode,
        kind: PermissionKind,
        route_path: Option<String>,
        component: Option<String>,
        icon: Option<String>,
        api_method: Option<ApiMethod>,
        api_path: Option<String>,
        remark: Option<String>,
        sort: Option<i32>,
        status: Option<Status>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::ensure_fields_match_kind(kind, &route_path, &component, &api_method, &api_path)?;

        Ok(Self {
            id,
            parent_id,
            name,
            code,
            kind,
            route_path,
            component,
            icon,
            api_method,
            api_path,
            is_builtin: false,
            remark,
            sort: sort.unwrap_or(1000),
            status: status.unwrap_or(Status::Enabled),
            audit_meta: AuditMeta::new(operator_id, now),
            delete_meta: DeleteMeta::new(),
            version_meta: VersionMeta::new(),
        })
    }

    /// 从数据库还原
    #[allow(clippy::too_many_arguments)]
    pub fn restore(
        id: PermissionId,
        parent_id: Option<PermissionId>,
        name: PermissionName,
        code: PermissionCode,
        kind: PermissionKind,
        route_path: Option<String>,
        component: Option<String>,
        icon: Option<String>,
        api_method: Option<ApiMethod>,
        api_path: Option<String>,
        is_builtin: bool,
        remark: Option<String>,
        sort: i32,
        status: Status,
        audit_meta: AuditMeta,
        delete_meta: DeleteMeta,
        version_meta: VersionMeta,
    ) -> Self {
        Self {
            id,
            parent_id,
            name,
            code,
            kind,
            route_path,
            component,
            icon,
            api_method,
            api_path,
            is_builtin,
            remark,
            sort,
            status,
            audit_meta,
            delete_meta,
            version_meta,
        }
    }

    /// 通用修改前置校验：内置、已删除拦截
    fn ensure_modifiable(&self) -> Result<(), DomainError> {
        if self.is_builtin {
            return Err(DomainError::PermissionProtected { id: self.id });
        }
        if self.delete_meta.is_deleted() {
            return Err(DomainError::PermissionNotFound { id: self.id });
        }
        Ok(())
    }

    /// 校验附属字段是否与权限类型匹配，拦截明显不合理的组合
    fn ensure_fields_match_kind(
        kind: PermissionKind,
        route_path: &Option<String>,
        component: &Option<String>,
        api_method: &Option<ApiMethod>,
        api_path: &Option<String>,
    ) -> Result<(), DomainError> {
        match kind {
            PermissionKind::Button => {
                if route_path.is_some()
                    || component.is_some()
                    || api_method.is_some()
                    || api_path.is_some()
                {
                    return Err(DomainError::PermissionKindFieldMismatch {
                        kind: kind.as_str(),
                        reason: "按钮类型权限不应填写菜单或接口专属字段",
                    });
                }
            }
            PermissionKind::Menu => {
                if api_method.is_some() || api_path.is_some() {
                    return Err(DomainError::PermissionKindFieldMismatch {
                        kind: kind.as_str(),
                        reason: "菜单类型权限不应填写接口专属字段",
                    });
                }
            }
            PermissionKind::Api => {
                if route_path.is_some() || component.is_some() {
                    return Err(DomainError::PermissionKindFieldMismatch {
                        kind: kind.as_str(),
                        reason: "接口类型权限不应填写菜单专属字段",
                    });
                }
            }
        }
        Ok(())
    }

    /// 更新权限基本信息（不含 parent_id，父级变更走 change_parent）
    #[allow(clippy::too_many_arguments)]
    pub fn update_info(
        &mut self,
        new_name: PermissionName,
        new_code: PermissionCode,
        new_kind: PermissionKind,
        new_route_path: Option<String>,
        new_component: Option<String>,
        new_icon: Option<String>,
        new_api_method: Option<ApiMethod>,
        new_api_path: Option<String>,
        new_remark: Option<String>,
        new_sort: Option<i32>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        Self::ensure_fields_match_kind(
            new_kind,
            &new_route_path,
            &new_component,
            &new_api_method,
            &new_api_path,
        )?;

        self.name = new_name;
        self.code = new_code;
        self.kind = new_kind;
        self.route_path = new_route_path;
        self.component = new_component;
        self.icon = new_icon;
        self.api_method = new_api_method;
        self.api_path = new_api_path;
        if let Some(remark) = new_remark {
            self.remark = Some(remark);
        }
        if let Some(sort) = new_sort {
            self.sort = sort;
        }
        self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 变更父级权限（树形结构调整）
    /// 仅拦截"设置为自身"这种直接自环；多级循环引用需由仓储层在调用前查询校验
    pub fn change_parent(
        &mut self,
        new_parent_id: Option<PermissionId>,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        if let Some(pid) = new_parent_id {
            if pid == self.id {
                return Err(DomainError::PermissionInvalidParent {
                    id: self.id,
                    reason: "不能将自己设置为自己的父级",
                });
            }
        }

        self.parent_id = new_parent_id;
        self.audit_meta.update(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 删除权限
    pub fn delete(
        &mut self,
        operator_id: Option<Uuid>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        self.ensure_modifiable()?;

        self.audit_meta.update(operator_id, now);
        self.delete_meta.delete(operator_id, now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 启用权限
    pub fn enable(&mut self, operator_id: Uuid, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status == Status::Enabled {
            return Err(DomainError::PermissionStatusAlreadyEnabled { id: self.id });
        }
        self.status = Status::Enabled;
        self.audit_meta.update(Some(operator_id), now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    /// 禁用权限
    pub fn disable(&mut self, operator_id: Uuid, now: DateTime<Utc>) -> Result<(), DomainError> {
        self.ensure_modifiable()?;
        if self.status == Status::Disabled {
            return Err(DomainError::PermissionStatusAlreadyDisabled { id: self.id });
        }
        self.status = Status::Disabled;
        self.audit_meta.update(Some(operator_id), now);
        self.version_meta = self.version_meta.next();
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------
    pub fn id(&self) -> PermissionId {
        self.id
    }
    pub fn parent_id(&self) -> Option<PermissionId> {
        self.parent_id
    }
    pub fn name(&self) -> &PermissionName {
        &self.name
    }
    pub fn code(&self) -> &PermissionCode {
        &self.code
    }
    pub fn kind(&self) -> PermissionKind {
        self.kind
    }
    pub fn route_path(&self) -> Option<&str> {
        self.route_path.as_deref()
    }
    pub fn component(&self) -> Option<&str> {
        self.component.as_deref()
    }
    pub fn icon(&self) -> Option<&str> {
        self.icon.as_deref()
    }
    pub fn api_method(&self) -> Option<ApiMethod> {
        self.api_method
    }
    pub fn api_path(&self) -> Option<&str> {
        self.api_path.as_deref()
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
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
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
mod permission_aggregate_tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 1, 1, 12, 0, 0).unwrap()
    }

    fn operator_uuid() -> Uuid {
        Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap()
    }

    fn test_permission_id() -> PermissionId {
        PermissionId::from_uuid(Uuid::now_v7())
    }

    fn menu_permission(operator: Option<Uuid>, now: DateTime<Utc>) -> Permission {
        Permission::new(
            test_permission_id(),
            None,
            PermissionName::new("用户管理").unwrap(),
            PermissionCode::new("iam:user:menu").unwrap(),
            PermissionKind::Menu,
            Some("/system/user".to_string()),
            Some("views/system/user/index".to_string()),
            Some("user-icon".to_string()),
            None,
            None,
            None,
            None,
            None,
            operator,
            now,
        )
        .unwrap()
    }

    fn api_permission(operator: Option<Uuid>, now: DateTime<Utc>) -> Permission {
        Permission::new(
            test_permission_id(),
            None,
            PermissionName::new("新增用户接口").unwrap(),
            PermissionCode::new("iam:user:add").unwrap(),
            PermissionKind::Api,
            None,
            None,
            None,
            Some(ApiMethod::Post),
            Some("/api/v1/users".to_string()),
            None,
            None,
            None,
            operator,
            now,
        )
        .unwrap()
    }

    fn builtin_permission(operator: Option<Uuid>, now: DateTime<Utc>) -> Permission {
        let mut p = menu_permission(operator, now);
        p.is_builtin = true;
        p
    }

    #[test]
    fn test_new_menu_default_values() {
        let now = test_now();
        let p = menu_permission(Some(operator_uuid()), now);

        assert!(!p.is_builtin());
        assert_eq!(p.sort(), 1000);
        assert_eq!(p.status(), Status::Enabled);
        assert!(p.is_root());
        assert_eq!(p.version_meta().value(), 0);
        assert_eq!(p.audit_meta().created_by(), Some(operator_uuid()));
        assert_eq!(p.route_path(), Some("/system/user"));
    }

    #[test]
    fn test_new_rejects_mismatched_kind_fields() {
        let now = test_now();
        // Button 类型却填了 api_path，应当被拒绝
        let err = Permission::new(
            test_permission_id(),
            None,
            PermissionName::new("按钮").unwrap(),
            PermissionCode::new("iam:user:btn").unwrap(),
            PermissionKind::Button,
            None,
            None,
            None,
            None,
            Some("/api/v1/users".to_string()),
            None,
            None,
            None,
            None,
            now,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            DomainError::PermissionKindFieldMismatch { .. }
        ));
    }

    #[test]
    fn test_update_info_success() {
        let now = test_now();
        let mut p = menu_permission(None, now);

        p.update_info(
            PermissionName::new("用户管理v2").unwrap(),
            PermissionCode::new("iam:user:menu:v2").unwrap(),
            PermissionKind::Menu,
            Some("/system/user/v2".to_string()),
            Some("views/system/user/v2".to_string()),
            None,
            None,
            None,
            Some("new remark".into()),
            Some(600),
            Some(operator_uuid()),
            now,
        )
        .unwrap();

        assert_eq!(p.name().as_str(), "用户管理v2");
        assert_eq!(p.sort(), 600);
        assert_eq!(p.route_path(), Some("/system/user/v2"));
        assert_eq!(p.version_meta().value(), 1);
    }

    #[test]
    fn test_update_info_fail_builtin() {
        let now = test_now();
        let mut p = builtin_permission(None, now);

        let err = p
            .update_info(
                PermissionName::new("x").unwrap(),
                PermissionCode::new("iam:x").unwrap(),
                PermissionKind::Menu,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(operator_uuid()),
                now,
            )
            .unwrap_err();

        assert!(matches!(err, DomainError::PermissionProtected { .. }));
    }

    #[test]
    fn test_update_info_rejects_mismatched_kind_fields() {
        let now = test_now();
        let mut p = api_permission(None, now);

        // 把类型切换成 Button，但仍然带着 Api 专属字段，应当被拒绝
        let err = p
            .update_info(
                PermissionName::new("按钮").unwrap(),
                PermissionCode::new("iam:user:btn2").unwrap(),
                PermissionKind::Button,
                None,
                None,
                None,
                Some(ApiMethod::Post),
                Some("/api/v1/users".to_string()),
                None,
                None,
                Some(operator_uuid()),
                now,
            )
            .unwrap_err();

        assert!(matches!(
            err,
            DomainError::PermissionKindFieldMismatch { .. }
        ));
    }

    #[test]
    fn test_change_parent_success() {
        let now = test_now();
        let mut p = menu_permission(None, now);
        let parent_id = PermissionId::from_uuid(Uuid::now_v7());

        p.change_parent(Some(parent_id), Some(operator_uuid()), now)
            .unwrap();

        assert_eq!(p.parent_id(), Some(parent_id));
        assert!(!p.is_root());
        assert_eq!(p.version_meta().value(), 1);
    }

    #[test]
    fn test_change_parent_reject_self_reference() {
        let now = test_now();
        let mut p = menu_permission(None, now);
        let self_id = p.id();

        let err = p
            .change_parent(Some(self_id), Some(operator_uuid()), now)
            .unwrap_err();

        assert!(matches!(err, DomainError::PermissionInvalidParent { .. }));
        assert!(p.is_root());
    }

    #[test]
    fn test_change_parent_fail_builtin() {
        let now = test_now();
        let mut p = builtin_permission(None, now);
        let parent_id = PermissionId::from_uuid(Uuid::now_v7());

        let err = p
            .change_parent(Some(parent_id), Some(operator_uuid()), now)
            .unwrap_err();

        assert!(matches!(err, DomainError::PermissionProtected { .. }));
        assert!(p.is_root());
    }

    #[test]
    fn test_enable_disable_flow() {
        let now = test_now();
        let mut p = Permission::new(
            test_permission_id(),
            None,
            PermissionName::new("测试").unwrap(),
            PermissionCode::new("iam:test").unwrap(),
            PermissionKind::Button,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(Status::Disabled),
            None,
            now,
        )
        .unwrap();

        p.enable(operator_uuid(), now).unwrap();
        assert_eq!(p.status(), Status::Enabled);

        let err = p.enable(operator_uuid(), now).unwrap_err();
        assert!(matches!(
            err,
            DomainError::PermissionStatusAlreadyEnabled { .. }
        ));

        p.disable(operator_uuid(), now).unwrap();
        assert_eq!(p.status(), Status::Disabled);

        let err = p.disable(operator_uuid(), now).unwrap_err();
        assert!(matches!(
            err,
            DomainError::PermissionStatusAlreadyDisabled { .. }
        ));
    }

    #[test]
    fn test_soft_delete_builtin_reject() {
        let now = test_now();
        let mut p = builtin_permission(None, now);

        let err = p.delete(Some(operator_uuid()), now).unwrap_err();
        assert!(matches!(err, DomainError::PermissionProtected { .. }));
        assert!(!p.delete_meta().is_deleted());
    }

    #[test]
    fn test_soft_delete_normal_success() {
        let now = test_now();
        let mut p = api_permission(None, now);

        p.delete(Some(operator_uuid()), now).unwrap();
        assert!(p.delete_meta().is_deleted());

        // 已删除后再次删除应返回 NotFound
        let err = p.delete(Some(operator_uuid()), now).unwrap_err();
        assert!(matches!(err, DomainError::PermissionNotFound { .. }));
    }
}
