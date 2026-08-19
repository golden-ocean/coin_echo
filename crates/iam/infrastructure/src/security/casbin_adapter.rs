//! Casbin Adapter 的 IAM 具体实现：从 Postgres 全量加载 g/p 策略。
//!
//! 这个适配器故意不实现"增量加载"和"通过 Casbin API 写策略"——
//! 策略的写入路径是 IAM 的 Command（RoleAssignPermissionsCommand /
//! UserAssignRolesCommand），直接操作 iam_user_role / iam_role_permission
//! 表；Casbin 这边只负责"全量重新读一遍"（reload_policy），不反向写回
//! 数据库。这样避免出现"Casbin 自己的写路径"和"IAM Command 的写路径"
//! 两条路可能互相打架的问题——数据库表是唯一数据源。

use async_trait::async_trait;
use casbin::{Adapter, Filter, Model, Result as CasbinResult};
use sqlx::PgPool;

pub struct CasbinAdapter {
    pool: PgPool,
}

impl CasbinAdapter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Adapter for CasbinAdapter {
    async fn load_policy(&mut self, m: &mut dyn Model) -> CasbinResult<()> {
        // g: user_id -> role_id
        let g_rows = sqlx::query!(r#"SELECT user_id, role_id FROM iam_user_role"#)
            .fetch_all(&self.pool)
            .await
            .map_err(adapter_err)?;

        for row in g_rows {
            m.add_policy(
                "g",
                "g",
                vec![row.user_id.to_string(), row.role_id.to_string()],
            );
        }

        // p: role_id -> permission_code
        // ⚠️ obj/act 目前用 permission.code 整个填进 obj，act 固定为 "*"——
        // 这是最简单的映射方式，前提是你的 PermissionCode（如
        // "iam:user:add"）已经能唯一标识"一个操作"，不需要再拆出单独的
        // act 维度。如果以后要做更细粒度的按 HTTP method 区分权限，
        // 这里要改成用 api_method/api_path 拼 obj+act
        let p_rows = sqlx::query!(
            r#"
                SELECT rp.role_id, p.code
                FROM iam_role_permission rp
                JOIN iam_permission p ON p.id = rp.permission_id
                WHERE p.deleted_at IS NULL AND p.status = 'enabled'
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(adapter_err)?;

        for row in p_rows {
            m.add_policy(
                "p",
                "p",
                vec![row.role_id.to_string(), row.code],
            );
        }

        Ok(())
    }

    async fn save_policy(&mut self, _m: &mut dyn Model) -> CasbinResult<()> {
        // 刻意不实现：策略写入只能通过 IAM Command 改数据库表 +
        // reload_policy() 重新拉取，不允许 Casbin 自己写回数据库。
        Ok(())
    }

    async fn load_filtered_policy<'a>(
        &mut self,
        _m: &mut dyn Model,
        _f: Filter<'a>,
    ) -> CasbinResult<()> {
        Err(casbin_error(
            "CasbinAdapter 不支持增量过滤加载，请用 load_policy 全量加载",
        ))
    }

    async fn add_policy(
        &mut self,
        _sec: &str,
        _ptype: &str,
        _rule: Vec<String>,
    ) -> CasbinResult<bool> {
        Ok(false)
    }

    async fn add_policies(
        &mut self,
        _sec: &str,
        _ptype: &str,
        _rules: Vec<Vec<String>>,
    ) -> CasbinResult<bool> {
        Ok(false)
    }

    async fn remove_policy(
        &mut self,
        _sec: &str,
        _ptype: &str,
        _rule: Vec<String>,
    ) -> CasbinResult<bool> {
        Ok(false)
    }

    async fn remove_policies(
        &mut self,
        _sec: &str,
        _ptype: &str,
        _rules: Vec<Vec<String>>,
    ) -> CasbinResult<bool> {
        Ok(false)
    }

    async fn remove_filtered_policy(
        &mut self,
        _sec: &str,
        _ptype: &str,
        _field_index: usize,
        _field_values: Vec<String>,
    ) -> CasbinResult<bool> {
        Ok(false)
    }

    fn is_filtered(&self) -> bool {
        false
    }

    async fn clear_policy(&mut self) -> CasbinResult<()> {
        Ok(())
    }

}

fn adapter_err(e: sqlx::Error) -> casbin::error::Error {
    casbin_error(&e.to_string())
}

fn casbin_error(msg: &str) -> casbin::error::Error {
    casbin::error::Error::from(casbin::error::AdapterError(Box::new(std::io::Error::new(
        std::io::ErrorKind::Other,
        msg.to_string(),
    ))))
}
