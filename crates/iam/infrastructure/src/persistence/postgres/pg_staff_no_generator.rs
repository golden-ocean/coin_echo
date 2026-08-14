use std::str::FromStr;

use iam_application::ports::{PortError, StaffNoGenerator};
use iam_domain::user::value_object::StaffNo;
use sqlx::PgPool;

pub struct PgStaffNoGenerator {
    pool: PgPool,
}

impl PgStaffNoGenerator {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl StaffNoGenerator for PgStaffNoGenerator {
    async fn generate(&self) -> Result<StaffNo, PortError> {
        // 假设使用 PostgreSQL 的 sequence 序号自增
        let row: (i64,) = sqlx::query_as("SELECT nextval('seq_iam_user_staff_no')")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| PortError::Infrastructure(e.to_string()))?;

        // 格式化工号（例如: STAFF-000001）
        let staff_no_str = format!("STAFF-{:06}", row.0);

        StaffNo::from_str(&staff_no_str).map_err(|_| PortError::ValueConvert {
            field: "staff_no",
            value: row.0.to_string(),
        })
    }
}
