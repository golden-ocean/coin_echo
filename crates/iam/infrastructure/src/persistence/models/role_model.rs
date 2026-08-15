use chrono::{DateTime, Utc};
use iam_application::ports::PortError;
use uuid::Uuid;

use iam_domain::{
    id::RoleId,
    role::{
        Role,
        value_object::{RoleCode, RoleName},
    },
};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status, VersionMeta};

/// 数据库 `iam_role` 表的持久化 Model (PO)
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RoleModel {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub is_builtin: bool,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub version: i64,
}

impl From<&Role> for RoleModel {
    fn from(role: &Role) -> Self {
        Self {
            id: role.id().as_uuid(),
            name: role.name().to_string(),
            code: role.code().to_string(),
            is_builtin: role.is_builtin(),
            remark: role.remark().map(|v| v.to_string()),
            sort: role.sort(),
            status: role.status().to_string(),
            created_at: role.audit_meta().created_at(),
            created_by: role.audit_meta().created_by(),
            updated_at: role.audit_meta().updated_at(),
            updated_by: role.audit_meta().updated_by(),
            deleted_at: role.delete_meta().deleted_at(),
            deleted_by: role.delete_meta().deleted_by(),
            version: role.version_meta().value(),
        }
    }
}

// 核心：基于引用的转换，避免消耗 RoleModel 的所有权
impl TryFrom<&RoleModel> for Role {
    type Error = PortError;

    fn try_from(model: &RoleModel) -> Result<Self, Self::Error> {
        let id = RoleId::from_uuid(model.id);

        // 统一全线使用 try_into() 风格，保持高一致性
        let name: RoleName =
            model
                .name
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "name",
                    value: model.name.clone(),
                })?;

        let code: RoleCode =
            model
                .code
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "code",
                    value: model.code.clone(),
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

        let version_meta: VersionMeta =
            model
                .version
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "version",
                    value: model.version.to_string(),
                })?;

        // 调用 Role::restore 完成聚合重构
        // 注意：权限列表需要由 Repository 在中间表单独加载填充
        Ok(Role::restore(
            id,
            name,
            code,
            model.is_builtin,
            model.remark.clone(),
            model.sort,
            status,
            Vec::new(),
            audit_meta,
            delete_meta,
            version_meta,
        ))
    }
}

// 顺手补充所有权版本的实现，内部直接委派给引用实现，兼顾两种调用方式
impl TryFrom<RoleModel> for Role {
    type Error = PortError;

    #[inline]
    fn try_from(model: RoleModel) -> Result<Self, Self::Error> {
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

    fn create_valid_role_model() -> RoleModel {
        let now = test_utc();
        let op_uuid = Uuid::now_v7();

        RoleModel {
            id: Uuid::now_v7(),
            name: "运营角色".to_string(),
            code: "op_role".to_string(),
            is_builtin: false,
            sort: 5,
            remark: Some("后台运营管理专用角色".to_string()),
            status: "enabled".to_string(),
            created_at: now,
            created_by: Some(op_uuid),
            updated_at: now,
            updated_by: Some(op_uuid),
            deleted_at: None,
            deleted_by: None,
            version: 1,
        }
    }

    #[test]
    fn test_role_model_ref_try_into_role_success() {
        let model = create_valid_role_model();
        // 测试借用 &RoleModel 进行转换
        let role_result: Result<Role, PortError> = (&model).try_into();

        assert!(role_result.is_ok());
        let role = role_result.unwrap();

        assert_eq!(role.id().as_uuid(), model.id);
        assert_eq!(role.name().to_string(), model.name);
        assert_eq!(role.code().to_string(), model.code);
        assert_eq!(role.status().to_string(), model.status);
        assert_eq!(role.sort(), model.sort);
    }

    #[test]
    fn test_role_roundtrip_conversion() {
        let original_model = create_valid_role_model();
        let role: Role = (&original_model).try_into().expect("转换应成功");

        let converted_model = RoleModel::from(&role);

        assert_eq!(original_model.id, converted_model.id);
        assert_eq!(original_model.name, converted_model.name);
        assert_eq!(original_model.code, converted_model.code);
        assert_eq!(original_model.status, converted_model.status);
        assert_eq!(original_model.version, converted_model.version);
    }

    #[test]
    fn test_invalid_name_triggers_port_error() {
        let mut model = create_valid_role_model();
        model.name = "管理员@123".to_string();

        let result: Result<Role, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "name");
                assert_eq!(value, "管理员@123");
            }
            _ => panic!("应该返回 name 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_status_triggers_port_error() {
        let mut model = create_valid_role_model();
        model.status = "open".to_string();

        let result: Result<Role, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "status");
                assert_eq!(value, "open");
            }
            _ => panic!("应该返回 status 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_version_triggers_port_error() {
        let mut model = create_valid_role_model();
        model.version = -1;

        let result: Result<Role, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "version");
                assert_eq!(value, "-1");
            }
            _ => panic!("应该返回 version 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_deleted_role_conversion() {
        let mut model = create_valid_role_model();
        let del_time = test_utc();
        let del_user = Uuid::now_v7();

        model.deleted_at = Some(del_time);
        model.deleted_by = Some(del_user);

        let role: Role = (&model).try_into().expect("软删除 Role 转换应成功");

        let del_meta = role.delete_meta();
        assert_eq!(del_meta.deleted_at(), Some(del_time));
        assert_eq!(del_meta.deleted_by(), Some(del_user));
    }
}
