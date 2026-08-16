use sqlx::{Postgres, Transaction};

use crate::persistence::models::UserModel;
use iam_application::ports::{PortError, UserRepository};
use iam_domain::id::UserId;
use iam_domain::user::{
    User,
    value_object::{Email, Phone},
};

mod constraints {
    pub const USERNAME: &str = "uk_iam_user_username";
    pub const EMAIL: &str = "uk_iam_user_email";
    pub const PHONE: &str = "uk_iam_user_phone";
    pub const STAFF_NO: &str = "uk_iam_user_staff_no";
}

/// UserRepository 的 PostgreSQL 基础设施层具体实现
pub struct PgUserRepository<'tx, 'c> {
    tx: &'tx mut Transaction<'c, Postgres>,
}

impl<'tx, 'c> PgUserRepository<'tx, 'c> {
    pub fn new(tx: &'tx mut Transaction<'c, Postgres>) -> Self {
        Self { tx }
    }

    fn map_sqlx_error(e: sqlx::Error) -> PortError {
        if let sqlx::Error::Database(db_err) = &e {
            if db_err.is_unique_violation() {
                return match db_err.constraint().unwrap_or_default() {
                    constraints::USERNAME => PortError::UniqueConflict {
                        entity: "user",
                        field: "username",
                    },
                    constraints::EMAIL => PortError::UniqueConflict {
                        entity: "user",
                        field: "email",
                    },
                    constraints::PHONE => PortError::UniqueConflict {
                        entity: "user",
                        field: "phone",
                    },
                    constraints::STAFF_NO => PortError::UniqueConflict {
                        entity: "user",
                        field: "staff_no",
                    },
                    other => {
                        PortError::Infrastructure(format!("unknown unique constraint: {other}"))
                    }
                };
            }
            if db_err
                .code()
                .map(|c| c == "40001" || c == "40P01")
                .unwrap_or(false)
            {
                return PortError::VersionConflict { entity: "user" };
            }
        }
        if matches!(e, sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed) {
            return PortError::Infrastructure(e.to_string());
        }
        PortError::Database
    }
}

#[async_trait::async_trait]
impl<'tx, 'c> UserRepository for PgUserRepository<'tx, 'c> {
    // ── 写入操作 ────────────────────────────────────────────────
    async fn insert(&mut self, user: &User) -> Result<(), PortError> {
        let m = UserModel::from(user);
        sqlx::query!(
            r#"
                INSERT INTO iam_user (
                    id, username, staff_no, name, email, phone, gender, birthday, avatar,
                    password_hash, password_updated_at, employment_status, data_scope,
                    is_builtin, sort, remark, status, organization_id, position_id,
                    created_at, created_by, updated_at, updated_by, version
                ) VALUES (
                    $1,$2,$3,$4,$5,$6,$7,$8,$9,
                    $10,$11,$12,$13,$14,$15,$16,$17,$18,$19,
                    $20,$21,$22,$23,$24
                )
            "#,
            m.id,
            m.username,
            m.staff_no,
            m.name,
            m.email,
            m.phone,
            m.gender,
            m.birthday,
            m.avatar,
            m.password_hash,
            m.password_updated_at,
            m.employment_status,
            m.data_scope,
            m.is_builtin,
            m.sort,
            m.remark,
            m.status,
            m.organization_id,
            m.position_id,
            m.created_at,
            m.created_by,
            m.updated_at,
            m.updated_by,
            m.version,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(())
    }

    async fn update(&mut self, user: &User) -> Result<(), PortError> {
        let m = UserModel::from(user);
        let old_version = m.version - 1;
        let result = sqlx::query!(
            r#"
                UPDATE iam_user SET
                    username = $2, staff_no = $3, name = $4, email = $5, phone = $6,
                    gender = $7, birthday = $8, avatar = $9,
                    password_hash = $10, password_updated_at = $11,
                    employment_status = $12, data_scope = $13,
                    sort = $14, remark = $15, status = $16,
                    organization_id = $17, position_id = $18,
                    updated_at = $19, updated_by = $20, version = $21
                WHERE id = $1 AND version = $22 AND deleted_at IS NULL
            "#,
            m.id,
            m.username,
            m.staff_no,
            m.name,
            m.email,
            m.phone,
            m.gender,
            m.birthday,
            m.avatar,
            m.password_hash,
            m.password_updated_at,
            m.employment_status,
            m.data_scope,
            m.sort,
            m.remark,
            m.status,
            m.organization_id,
            m.position_id,
            m.updated_at,
            m.updated_by,
            m.version,
            old_version
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::VersionConflict { entity: "user" });
        }
        Ok(())
    }

    async fn soft_delete(&mut self, user: &User) -> Result<(), PortError> {
        let m = UserModel::from(user);
        let old_version = m.version - 1;
        let result = sqlx::query!(
            r#"
                UPDATE iam_user SET
                    deleted_at = $2, deleted_by = $3,
                    updated_at = $4, updated_by = $5, version = $6
                WHERE id = $1 AND version = $7 AND deleted_at IS NULL
            "#,
            m.id,
            m.deleted_at,
            m.deleted_by,
            m.updated_at,
            m.updated_by,
            m.version,
            old_version,
        )
        .execute(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        if result.rows_affected() == 0 {
            return Err(PortError::VersionConflict { entity: "user" });
        }
        Ok(())
    }

    // ── 聚合根查询 ───────────────
    async fn find_by_id(&mut self, id: &UserId) -> Result<Option<User>, PortError> {
        let row = sqlx::query_as!(
            UserModel,
            r#"
                SELECT
                    id, username, staff_no, name, email, phone, gender, birthday, avatar,
                    password_hash, password_updated_at, employment_status, data_scope,
                    is_builtin, sort, remark, status, organization_id, position_id,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by, version
                FROM iam_user
                WHERE id = $1 AND deleted_at IS NULL
            "#,
            id.as_uuid()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_username(&mut self, username: &str) -> Result<Option<User>, PortError> {
        let row = sqlx::query_as!(
            UserModel,
            r#"
                SELECT
                    id, username, staff_no, name, email, phone, gender, birthday, avatar,
                    password_hash, password_updated_at, employment_status, data_scope,
                    is_builtin, sort, remark, status, organization_id, position_id,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by, version
                FROM iam_user
                WHERE username = $1 AND deleted_at IS NULL
            "#,
            username
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_email(&mut self, email: &Email) -> Result<Option<User>, PortError> {
        let row = sqlx::query_as!(
            UserModel,
            r#"
                SELECT
                    id, username, staff_no, name, email, phone, gender, birthday, avatar,
                    password_hash, password_updated_at, employment_status, data_scope,
                    is_builtin, sort, remark, status, organization_id, position_id,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by, version
                FROM iam_user
                WHERE email = $1 AND deleted_at IS NULL
            "#,
            email.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    async fn find_by_phone(&mut self, phone: &Phone) -> Result<Option<User>, PortError> {
        let row = sqlx::query_as!(
            UserModel,
            r#"
                SELECT
                    id, username, staff_no, name, email, phone, gender, birthday, avatar,
                    password_hash, password_updated_at, employment_status, data_scope,
                    is_builtin, sort, remark, status, organization_id, position_id,
                    created_at, created_by, updated_at, updated_by, deleted_at, deleted_by, version
                FROM iam_user
                WHERE phone = $1 AND deleted_at IS NULL
            "#,
            phone.as_str()
        )
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        row.map(TryInto::try_into).transpose()
    }

    // ── 存在性检查 (query_scalar! 零结构体) ────────────────────
    async fn exists_by_username(&mut self, username: &str) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_user WHERE username = $1 AND deleted_at IS NULL) as "exists!""#,
            username
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_email(&mut self, email: &Email) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_user WHERE email = $1 AND deleted_at IS NULL) as "exists!""#,
            email.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }

    async fn exists_by_phone(&mut self, phone: &Phone) -> Result<bool, PortError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(SELECT 1 FROM iam_user WHERE phone = $1 AND deleted_at IS NULL) as "exists!""#,
            phone.as_str()
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(Self::map_sqlx_error)?;

        Ok(result)
    }
}
