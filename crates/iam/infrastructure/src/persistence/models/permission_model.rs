use std::str::FromStr;

use chrono::{DateTime, Utc};
use uuid::Uuid;

use iam_application::ports::PortError;
use iam_domain::{
    id::PermissionId,
    permission::{
        Permission,
        value_object::{ApiMethod, PermissionCode, PermissionKind, PermissionName},
    },
};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status};

/// `iam_permission` 表的数据库行模型，负责领域对象 <-> 数据库行的双向转换。
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PermissionModel {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub code: String,
    pub kind: String,
    pub route_path: Option<String>,
    pub component: Option<String>,
    pub icon: Option<String>,
    pub api_method: Option<String>,
    pub api_path: Option<String>,
    pub is_builtin: bool,
    pub remark: Option<String>,
    pub sort: i32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
}

impl From<&Permission> for PermissionModel {
    fn from(p: &Permission) -> Self {
        Self {
            id: p.id().as_uuid(),
            parent_id: p.parent_id().map(|pid| pid.as_uuid()),
            name: p.name().as_str().to_string(),
            code: p.code().as_str().to_string(),
            kind: p.kind().as_str().to_string(),
            route_path: p.route_path().map(str::to_string),
            component: p.component().map(str::to_string),
            icon: p.icon().map(str::to_string),
            api_method: p.api_method().map(|m| m.as_str().to_string()),
            api_path: p.api_path().map(str::to_string),
            is_builtin: p.is_builtin(),
            remark: p.remark().map(str::to_string),
            sort: p.sort(),
            status: p.status().to_string(),
            created_at: p.audit_meta().created_at(),
            created_by: p.audit_meta().created_by(),
            updated_at: p.audit_meta().updated_at(),
            updated_by: p.audit_meta().updated_by(),
            deleted_at: p.delete_meta().deleted_at(),
            deleted_by: p.delete_meta().deleted_by(),
        }
    }
}

// 核心：基于引用的转换，避免消耗 PermissionModel 的所有权
impl TryFrom<&PermissionModel> for Permission {
    type Error = PortError;

    fn try_from(model: &PermissionModel) -> Result<Self, Self::Error> {
        let id = PermissionId::from_uuid(model.id);
        let parent_id = model.parent_id.map(PermissionId::from_uuid);

        let name: PermissionName =
            model
                .name
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "name",
                    value: model.name.clone(),
                })?;

        let code: PermissionCode =
            model
                .code
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "code",
                    value: model.code.clone(),
                })?;

        let kind: PermissionKind =
            model
                .kind
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "kind",
                    value: model.kind.clone(),
                })?;

        let api_method: Option<ApiMethod> = model
            .api_method
            .as_deref()
            .map(ApiMethod::from_str)
            .transpose()
            .map_err(|_| PortError::ValueConvert {
                field: "api_method",
                value: model.api_method.clone().unwrap_or_default(),
            })?;

        let status: Status =
            model
                .status
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "status",
                    value: model.status.clone(),
                })?;

        let audit_meta = AuditMeta::restore(
            model.created_at,
            model.updated_at,
            model.created_by,
            model.updated_by,
        );
        let delete_meta = DeleteMeta::restore(model.deleted_at, model.deleted_by);

        // 注意：Permission::restore 不会重新触发 ensure_fields_match_kind 这类
        // 业务级一致性校验（那是 new/update_info 的职责），因为持久化数据一旦
        // 落库，应当被视为历史上已经通过过校验的合法状态，Mapper 只负责结构转换。
        Ok(Permission::restore(
            id,
            parent_id,
            name,
            code,
            kind,
            model.route_path.clone(),
            model.component.clone(),
            model.icon.clone(),
            api_method,
            model.api_path.clone(),
            model.is_builtin,
            model.remark.clone(),
            model.sort,
            status,
            audit_meta,
            delete_meta,
        ))
    }
}

impl TryFrom<PermissionModel> for Permission {
    type Error = PortError;

    #[inline]
    fn try_from(model: PermissionModel) -> Result<Self, Self::Error> {
        Self::try_from(&model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn test_utc() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 23, 10, 30, 0).unwrap()
    }

    // 构造一个"叶子权限"(带完整 api 信息)的合法 Model
    // 注意：PermissionName 只允许中文/字母/数字/下划线，不能带冒号、空格
    fn create_valid_api_permission_model() -> PermissionModel {
        let now = test_utc();
        let op_uuid = Uuid::now_v7();

        PermissionModel {
            id: Uuid::now_v7(),
            parent_id: Some(Uuid::now_v7()),
            name: "用户查询接口".to_string(),
            code: "iam:user:query".to_string(),
            kind: "api".to_string(),
            route_path: None,
            component: None,
            icon: None,
            api_method: Some("GET".to_string()),
            api_path: Some("/api/v1/users".to_string()),
            is_builtin: false,
            remark: Some("用户列表查询接口".to_string()),
            sort: 1,
            status: "enabled".to_string(),
            created_at: now,
            created_by: Some(op_uuid),
            updated_at: now,
            updated_by: Some(op_uuid),
            deleted_at: None,
            deleted_by: None,
        }
    }

    // 构造一个"顶层菜单权限"(可选字段基本为 None)的合法 Model
    fn create_valid_menu_permission_model() -> PermissionModel {
        let now = test_utc();

        PermissionModel {
            id: Uuid::now_v7(),
            parent_id: None,
            name: "系统管理".to_string(),
            code: "iam:system:manage".to_string(),
            kind: "menu".to_string(),
            route_path: Some("/system".to_string()),
            component: Some("Layout".to_string()),
            icon: Some("setting".to_string()),
            api_method: None,
            api_path: None,
            is_builtin: true,
            remark: None,
            sort: 0,
            status: "enabled".to_string(),
            created_at: now,
            created_by: None,
            updated_at: now,
            updated_by: None,
            deleted_at: None,
            deleted_by: None,
        }
    }

    #[test]
    fn test_api_permission_ref_try_into_success() {
        let model = create_valid_api_permission_model();
        let permission_result: Result<Permission, PortError> = (&model).try_into();

        assert!(permission_result.is_ok());
        let permission = permission_result.unwrap();

        assert_eq!(permission.id().as_uuid(), model.id);
        assert_eq!(
            permission.parent_id().map(|pid| pid.as_uuid()),
            model.parent_id
        );
        assert_eq!(permission.name().as_str(), model.name);
        assert_eq!(permission.code().as_str(), model.code);
        assert_eq!(permission.kind().as_str(), model.kind);
        assert_eq!(
            permission.api_method().map(|m| m.as_str().to_string()),
            model.api_method
        );
        assert_eq!(permission.api_path().map(str::to_string), model.api_path);
        assert_eq!(permission.sort(), model.sort);
    }

    #[test]
    fn test_menu_permission_optional_fields_all_none_success() {
        // 覆盖顶层菜单场景：parent_id / api_method / api_path / remark 全为 None
        let model = create_valid_menu_permission_model();
        let permission_result: Result<Permission, PortError> = (&model).try_into();

        assert!(permission_result.is_ok());
        let permission = permission_result.unwrap();

        assert!(permission.parent_id().is_none());
        assert!(permission.is_root());
        assert!(permission.api_method().is_none());
        assert!(permission.api_path().is_none());
        assert!(permission.remark().is_none());
        assert!(permission.is_builtin());
        assert_eq!(
            permission.route_path().map(str::to_string),
            model.route_path
        );
        assert_eq!(permission.component().map(str::to_string), model.component);
        assert_eq!(permission.icon().map(str::to_string), model.icon);
    }

    #[test]
    fn test_permission_roundtrip_conversion() {
        let original_model = create_valid_api_permission_model();
        let permission: Permission = (&original_model).try_into().expect("转换应成功");

        let converted_model = PermissionModel::from(&permission);

        assert_eq!(original_model.id, converted_model.id);
        assert_eq!(original_model.parent_id, converted_model.parent_id);
        assert_eq!(original_model.name, converted_model.name);
        assert_eq!(original_model.code, converted_model.code);
        assert_eq!(original_model.kind, converted_model.kind);
        assert_eq!(original_model.api_method, converted_model.api_method);
        assert_eq!(original_model.api_path, converted_model.api_path);
        assert_eq!(original_model.status, converted_model.status);
    }

    #[test]
    fn test_menu_permission_roundtrip_conversion() {
        // 额外覆盖一次可选字段全 None 场景下的往返一致性
        let original_model = create_valid_menu_permission_model();
        let permission: Permission = (&original_model).try_into().expect("转换应成功");

        let converted_model = PermissionModel::from(&permission);

        assert_eq!(original_model.route_path, converted_model.route_path);
        assert_eq!(original_model.component, converted_model.component);
        assert_eq!(original_model.icon, converted_model.icon);
        assert_eq!(original_model.api_method, converted_model.api_method);
        assert_eq!(original_model.remark, converted_model.remark);
        assert_eq!(original_model.is_builtin, converted_model.is_builtin);
    }

    #[test]
    fn test_owned_try_from_delegates_correctly() {
        // 覆盖所有权版本的 TryFrom<PermissionModel>
        let model = create_valid_api_permission_model();
        let expected: Permission = (&model).try_into().expect("引用版本转换应成功");

        let permission: Permission = model.try_into().expect("所有权版本转换应成功");

        assert_eq!(permission.id(), expected.id());
        assert_eq!(permission.code().as_str(), expected.code().as_str());
    }

    #[test]
    fn test_invalid_name_triggers_port_error() {
        let mut model = create_valid_api_permission_model();
        // 冒号在 PermissionName 里非法（只允许中文/字母/数字/下划线）
        model.name = "查询:接口".to_string();

        let result: Result<Permission, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "name");
                assert_eq!(value, "查询:接口");
            }
            _ => panic!("应该返回 name 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_code_triggers_port_error() {
        let mut model = create_valid_api_permission_model();
        // PermissionCode 只允许小写字母/数字/下划线/冒号/短横线，大写非法
        model.code = "IAM:USER:QUERY".to_string();

        let result: Result<Permission, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "code");
                assert_eq!(value, "IAM:USER:QUERY");
            }
            _ => panic!("应该返回 code 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_kind_triggers_port_error() {
        let mut model = create_valid_api_permission_model();
        model.kind = "unknown_kind".to_string();

        let result: Result<Permission, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "kind");
                assert_eq!(value, "unknown_kind");
            }
            _ => panic!("应该返回 kind 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_status_triggers_port_error() {
        let mut model = create_valid_api_permission_model();
        model.status = "open".to_string();

        let result: Result<Permission, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "status");
                assert_eq!(value, "open");
            }
            _ => panic!("应该返回 status 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_api_method_triggers_port_error() {
        // 专项覆盖新增的 api_method map_err 分支
        // ApiMethod::from_str 内部会先转大写，所以 "fetch" 不属于任何合法方法
        let mut model = create_valid_api_permission_model();
        model.api_method = Some("FETCH".to_string());

        let result: Result<Permission, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "api_method");
                assert_eq!(value, "FETCH");
            }
            _ => panic!("应该返回 api_method 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_api_method_none_does_not_trigger_error() {
        // api_method 为 None 是合法状态（如 Menu/Button 类型权限），不应报错
        let model = create_valid_menu_permission_model();
        let result: Result<Permission, PortError> = (&model).try_into();

        assert!(result.is_ok());
        assert!(result.unwrap().api_method().is_none());
    }

    #[test]
    fn test_deleted_permission_conversion() {
        let mut model = create_valid_api_permission_model();
        let del_time = test_utc();
        let del_user = Uuid::now_v7();

        model.deleted_at = Some(del_time);
        model.deleted_by = Some(del_user);

        let permission: Permission = (&model).try_into().expect("软删除 Permission 转换应成功");

        let del_meta = permission.delete_meta();
        assert_eq!(del_meta.deleted_at(), Some(del_time));
        assert_eq!(del_meta.deleted_by(), Some(del_user));
    }
}
