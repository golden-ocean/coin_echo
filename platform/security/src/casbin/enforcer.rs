//! Casbin enforcer 封装。

use casbin::{CoreApi, DefaultModel, Enforcer, FileAdapter};
use tokio::sync::RwLock;

use crate::casbin::config::CasbinConfig;
use crate::casbin::error::CasbinError;

/// Casbin enforcer 的并发安全封装。
///
/// Casbin 的 `Enforcer` 本身不是自带并发保护，多个请求并发调用 `check`
/// 需要外部加锁；策略几乎只读、偶尔写（管理员改权限），用
/// `tokio::sync::RwLock` 而非 `Mutex`，让并发的 `check` 调用之间不互相阻塞。
///
/// 手写 `Debug`：`casbin::Enforcer` 未实现 `Debug`，且策略数据本身也
/// 不适合被直接打进日志。
pub struct CasbinEnforcer {
    inner: RwLock<Enforcer>,
}

impl std::fmt::Debug for CasbinEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CasbinEnforcer").finish_non_exhaustive()
    }
}

impl CasbinEnforcer {
    /// 由配置构造，加载 model + policy 文件。
    ///
    /// 显式构造 [`DefaultModel`]/[`FileAdapter`] 而非把路径字符串直接传给
    /// `Enforcer::new`：后者要求入参实现 `TryIntoModel`/`TryIntoAdapter`，
    /// `String` 本身没有这两个实现，`&str` 虽有实现但生命周期绑定在
    /// `config` 上、无法满足内部 `Box<dyn Model>`/`Box<dyn Adapter>` 要求的
    /// `'static`。`DefaultModel::from_file`/`FileAdapter::new` 各自把文件
    /// 内容/路径读进自己拥有的数据，天然满足 `'static`。
    pub async fn new(config: &CasbinConfig) -> Result<Self, CasbinError> {
        let model = DefaultModel::from_file(config.model_path.clone())
            .await
            .map_err(|e| CasbinError::InitFailed(e.to_string()))?;
        let adapter = FileAdapter::new(config.policy_path.clone());

        let enforcer = Enforcer::new(model, adapter)
            .await
            .map_err(|e| CasbinError::InitFailed(e.to_string()))?;

        Ok(Self {
            inner: RwLock::new(enforcer),
        })
    }

    /// 校验 `subject` 是否可对 `object` 执行 `action`。
    ///
    /// 返回 `Ok(())` 表示允许；`Err(CasbinError::PermissionDenied)` 表示
    /// 拒绝。不用 `Result<bool, _>`——调用方本来就是要么继续处理请求、要么
    /// 直接返回 403，用 `?` 直接短路比每次手写
    /// `if !allowed { return Err(..) }` 更不容易漏判断。
    pub async fn check(
        &self,
        subject: &str,
        object: &str,
        action: &str,
    ) -> Result<(), CasbinError> {
        let enforcer = self.inner.read().await;
        let allowed = enforcer
            .enforce((subject, object, action))
            .map_err(|e| CasbinError::PolicyOperationFailed(e.to_string()))?;
        if allowed {
            Ok(())
        } else {
            Err(CasbinError::PermissionDenied)
        }
    }

    /// 运行期重新加载策略（如管理员改权限后，无需重启进程生效）。
    pub async fn reload_policy(&self) -> Result<(), CasbinError> {
        use casbin::MgmtApi;
        let mut enforcer = self.inner.write().await;
        enforcer
            .load_policy()
            .await
            .map_err(|e| CasbinError::PolicyOperationFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_config() -> (tempfile::TempDir, CasbinConfig) {
        let dir = tempfile::tempdir().unwrap();

        let model_path = dir.path().join("model.conf");
        let mut model_file = std::fs::File::create(&model_path).unwrap();
        write!(
            model_file,
            "[request_definition]\nr = sub, obj, act\n\n\
             [policy_definition]\np = sub, obj, act\n\n\
             [policy_effect]\ne = some(where (p.eft == allow))\n\n\
             [matchers]\nm = r.sub == p.sub && r.obj == p.obj && r.act == p.act\n"
        )
        .unwrap();

        let policy_path = dir.path().join("policy.csv");
        let mut policy_file = std::fs::File::create(&policy_path).unwrap();
        writeln!(policy_file, "p, alice, /v1/users, read").unwrap();

        let config = CasbinConfig {
            model_path: model_path.to_string_lossy().to_string(),
            policy_path: policy_path.to_string_lossy().to_string(),
        };
        (dir, config)
    }

    #[tokio::test]
    async fn allowed_action_passes_check() {
        let (_dir, config) = temp_config();
        let enforcer = CasbinEnforcer::new(&config).await.unwrap();
        assert!(enforcer.check("alice", "/v1/users", "read").await.is_ok());
    }

    #[tokio::test]
    async fn action_not_in_policy_is_denied() {
        let (_dir, config) = temp_config();
        let enforcer = CasbinEnforcer::new(&config).await.unwrap();
        let result = enforcer.check("alice", "/v1/users", "delete").await;
        assert!(matches!(result, Err(CasbinError::PermissionDenied)));
    }

    #[tokio::test]
    async fn unknown_subject_is_denied() {
        let (_dir, config) = temp_config();
        let enforcer = CasbinEnforcer::new(&config).await.unwrap();
        let result = enforcer.check("bob", "/v1/users", "read").await;
        assert!(matches!(result, Err(CasbinError::PermissionDenied)));
    }

    #[tokio::test]
    async fn missing_model_file_fails_construction() {
        let config = CasbinConfig {
            model_path: "/nonexistent/model.conf".into(),
            policy_path: "/nonexistent/policy.csv".into(),
        };
        let result = CasbinEnforcer::new(&config).await;
        assert!(matches!(result, Err(CasbinError::InitFailed(_))));
    }

    #[tokio::test]
    async fn reload_picks_up_externally_modified_policy_file() {
        let (dir, config) = temp_config();
        let enforcer = CasbinEnforcer::new(&config).await.unwrap();

        assert!(enforcer.check("bob", "/v1/users", "read").await.is_err());

        let policy_path = dir.path().join("policy.csv");
        let mut policy_file = std::fs::OpenOptions::new()
            .append(true)
            .open(&policy_path)
            .unwrap();
        writeln!(policy_file, "p, bob, /v1/users, read").unwrap();

        enforcer.reload_policy().await.unwrap();
        assert!(enforcer.check("bob", "/v1/users", "read").await.is_ok());
    }
}
