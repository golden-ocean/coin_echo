use chrono::{DateTime, NaiveDate, Utc};
use uuid::Uuid;

use iam_application::ports::PortError;
use iam_domain::{
    id::{OrganizationId, PositionId, UserId},
    user::{
        User,
        value_object::{
            DataScope, Email, EmploymentStatus, Gender, PasswordCredential, Phone, StaffNo,
        },
    },
};
use platform_kernel::meta::{AuditMeta, DeleteMeta, Status, VersionMeta};

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct UserModel {
    pub id: Uuid,
    pub username: String,
    pub staff_no: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub gender: String,
    pub birthday: Option<NaiveDate>,
    pub avatar: Option<String>,
    pub password_hash: String,
    pub password_updated_at: DateTime<Utc>,
    pub employment_status: String,
    pub data_scope: String,
    pub is_builtin: bool,
    pub sort: i32,
    pub remark: Option<String>,
    pub status: String,
    pub organization_id: Option<Uuid>,
    pub position_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<Uuid>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<Uuid>,
    pub version: i64,
}

impl From<&User> for UserModel {
    fn from(user: &User) -> Self {
        Self {
            id: user.id().as_uuid(),
            username: user.username().to_string(),
            staff_no: user.staff_no().as_str().to_string(),
            name: user.name().to_string(),
            email: user.email().as_str().to_string(),
            phone: user.phone().as_str().to_string(),
            gender: user.gender().to_string(),
            birthday: user.birthday(),
            avatar: user.avatar().map(|s| s.to_string()),
            password_hash: user.password_credential().hash_as_str().to_string(),
            password_updated_at: user.password_credential().updated_at(),
            employment_status: user.employment_status().to_string(),
            data_scope: user.data_scope().to_string(),
            is_builtin: user.is_builtin(),
            sort: user.sort(),
            remark: user.remark().map(|s| s.to_string()),
            status: user.status().to_string(),
            organization_id: user.organization_id().map(|v| v.as_uuid()),
            position_id: user.position_id().map(|v| v.as_uuid()),
            created_at: user.audit_meta().created_at(),
            created_by: user.audit_meta().created_by(),
            updated_at: user.audit_meta().updated_at(),
            updated_by: user.audit_meta().updated_by(),
            deleted_at: user.delete_meta().deleted_at(),
            deleted_by: user.delete_meta().deleted_by(),
            version: user.version_meta().value(),
        }
    }
}

// 核心：基于引用的转换，避免消耗 UserModel 的所有权
impl TryFrom<&UserModel> for User {
    type Error = PortError;

    fn try_from(model: &UserModel) -> Result<Self, Self::Error> {
        let id = UserId::from_uuid(model.id);

        // 统一全线使用 try_into() 风格，保持高一致性
        let staff_no: StaffNo =
            model
                .staff_no
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "staff_no",
                    value: model.staff_no.clone(),
                })?;

        let email: Email =
            model
                .email
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "email",
                    value: model.email.clone(),
                })?;

        let phone: Phone =
            model
                .phone
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "phone",
                    value: model.phone.clone(),
                })?;

        let gender: Gender =
            model
                .gender
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "gender",
                    value: model.gender.clone(),
                })?;

        let employment_status: EmploymentStatus = model
            .employment_status
            .as_str()
            .try_into()
            .map_err(|_| PortError::ValueConvert {
                field: "employment_status",
                value: model.employment_status.clone(),
            })?;

        let data_scope: DataScope =
            model
                .data_scope
                .as_str()
                .try_into()
                .map_err(|_| PortError::ValueConvert {
                    field: "data_scope",
                    value: model.data_scope.clone(),
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

        let password_cred =
            PasswordCredential::new(&model.password_hash, model.password_updated_at).map_err(
                |_| PortError::ValueConvert {
                    field: "password_hash",
                    value: model.password_hash.clone(),
                },
            )?;

        let org_id = model.organization_id.map(OrganizationId::from_uuid);
        let pos_id = model.position_id.map(PositionId::from_uuid);

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

        Ok(User::restore(
            id,
            model.username.clone(),
            staff_no,
            model.name.clone(),
            email,
            phone,
            gender,
            model.birthday,
            model.avatar.clone(),
            password_cred,
            employment_status,
            data_scope,
            model.is_builtin,
            model.sort,
            model.remark.clone(),
            status,
            org_id,
            pos_id,
            Vec::new(),
            audit_meta,
            delete_meta,
            version_meta,
        ))
    }
}

// 顺手补充所有权版本的实现，内部直接委派给引用实现，兼顾两种调用方式
impl TryFrom<UserModel> for User {
    type Error = PortError;

    #[inline]
    fn try_from(model: UserModel) -> Result<Self, Self::Error> {
        Self::try_from(&model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_user_model() -> UserModel {
        let now = Utc::now();
        UserModel {
            id: Uuid::now_v7(),
            username: "johndoe".to_string(),
            staff_no: "STAFF-000001".to_string(),
            name: "John Doe".to_string(),
            email: "john.doe@example.com".to_string(),
            phone: "13800138000".to_string(),
            gender: "male".to_string(),
            birthday: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
            avatar: Some("https://example.com/avatar.jpg".to_string()),
            password_hash: "$2b$12$eImiTXuWVxfM37uY4JANjO5E.A4Y2E3G8J6hO6J.1O5J".to_string(),
            password_updated_at: now,
            employment_status: "active".to_string(),
            data_scope: "all".to_string(),
            is_builtin: false,
            sort: 1,
            remark: Some("test remark".to_string()),
            status: "enabled".to_string(),
            organization_id: Some(Uuid::now_v7()),
            position_id: Some(Uuid::now_v7()),
            created_at: now,
            created_by: Some(Uuid::now_v7()),
            updated_at: now,
            updated_by: Some(Uuid::now_v7()),
            deleted_at: None,
            deleted_by: None,
            version: 1,
        }
    }

    #[test]
    fn test_user_model_ref_try_into_user_success() {
        let model = create_valid_user_model();
        // 测试借用 &UserModel 进行转换
        let user_result: Result<User, PortError> = (&model).try_into();

        assert!(user_result.is_ok());
        let user = user_result.unwrap();

        assert_eq!(user.id().as_uuid(), model.id);
        assert_eq!(user.staff_no().as_str(), model.staff_no);
        assert_eq!(user.email().as_str(), model.email);
        assert_eq!(user.phone().as_str(), model.phone);
        assert_eq!(user.gender().as_str(), model.gender);
        assert_eq!(user.employment_status().as_str(), model.employment_status);
    }

    #[test]
    fn test_user_roundtrip_conversion() {
        let original_model = create_valid_user_model();
        let user: User = (&original_model).try_into().expect("转换应成功");

        let converted_model = UserModel::from(&user);

        assert_eq!(original_model.id, converted_model.id);
        assert_eq!(original_model.staff_no, converted_model.staff_no);
        assert_eq!(original_model.email, converted_model.email);
        assert_eq!(original_model.phone, converted_model.phone);
        assert_eq!(original_model.gender, converted_model.gender);
        assert_eq!(
            original_model.employment_status,
            converted_model.employment_status
        );
        assert_eq!(original_model.version, converted_model.version);
    }

    #[test]
    fn test_invalid_value_objects_trigger_port_error() {
        let mut model = create_valid_user_model();
        model.staff_no = "INVALID-STAFF-NO".to_string();

        let result: Result<User, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "staff_no");
                assert_eq!(value, "INVALID-STAFF-NO");
            }
            _ => panic!("应该返回 StaffNo 的 ValueConvert 转换错误"),
        }
    }

    #[test]
    fn test_invalid_version_triggers_port_error() {
        let mut model = create_valid_user_model();
        model.version = -1;

        let result: Result<User, PortError> = (&model).try_into();

        match result {
            Err(PortError::ValueConvert { field, value }) => {
                assert_eq!(field, "version");
                assert_eq!(value, "-1");
            }
            _ => panic!("应该返回 version 的 ValueConvert 转换错误"),
        }
    }
}
