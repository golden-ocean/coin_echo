use casbin::{CoreApi, DefaultModel, Enforcer};
use tokio::sync::RwLock;

use crate::casbin::error::CasbinError;

pub const RBAC_MODEL: &str = r#"
[request_definition]
r = sub, obj

[policy_definition]
p = sub, obj

[role_definition]
g = _, _

[policy_effect]
e = some(where (p.eft == allow))

[matchers]
m = g(r.sub, p.sub) && r.obj == p.obj
"#;

/// Casbin enforcer 的并发安全封装。
///
/// 不认识策略数据从哪来——`adapter` 由调用方构造好传进来。生产环境传
/// `iam-infrastructure::casbin::IamCasbinAdapter`（从 Postgres 全量加载
/// g/p 策略）；测试/本地调试可以传 `casbin::FileAdapter`。这样
/// `platform-security` 保持领域无关，不需要认识任何业务表结构。
pub struct CasbinEnforcer {
    inner: RwLock<Enforcer>,
}

impl std::fmt::Debug for CasbinEnforcer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CasbinEnforcer").finish_non_exhaustive()
    }
}

impl CasbinEnforcer {
    /// 使用内嵌 RBAC 模型 + 自定义适配器构造。
    pub async fn with_adapter<A: casbin::Adapter + 'static>(
        adapter: A,
    ) -> Result<Self, CasbinError> {
        // 1. 从内存中的字符串文本构造 Model 实例
        let model = DefaultModel::from_str(RBAC_MODEL)
            .await
            .map_err(|e| CasbinError::InitFailed(e.to_string()))?;

        // 2. 使用 Model 实例和 Adapter 创建 Enforcer
        let enforcer = Enforcer::new(model, adapter)
            .await
            .map_err(|e| CasbinError::InitFailed(e.to_string()))?;

        Ok(Self {
            inner: RwLock::new(enforcer),
        })
    }

    // /// 使用外部模型文件 + 自定义适配器构造。
    // pub async fn from_file<A: casbin::Adapter + 'static>(
    //     model_path: &str,
    //     adapter: A,
    // ) -> Result<Self, CasbinError> {
    //     let model = DefaultModel::from_file(model_path)
    //         .await
    //         .map_err(|e| CasbinError::InitFailed(e.to_string()))?;
    //     let enforcer = Enforcer::new(model, adapter)
    //         .await
    //         .map_err(|e| CasbinError::InitFailed(e.to_string()))?;
    //     Ok(Self {
    //         inner: RwLock::new(enforcer),
    //     })
    // }

    /// 校验 `subject` 是否对应 `object`。
    pub async fn check(&self, subject: &str, object: &str) -> Result<(), CasbinError> {
        let enforcer = self.inner.read().await;
        // 临时最高权限账户
        if subject == "00000000-0000-0000-0000-000000000000" {
            return Ok(());
        }
        let allowed = enforcer
            .enforce((subject, object))
            .map_err(|e| CasbinError::PolicyOperationFailed(e.to_string()))?;
        if allowed {
            Ok(())
        } else {
            Err(CasbinError::PermissionDenied)
        }
    }

    /// 运行期重新加载策略——IAM 的角色/权限分配 Command 提交成功后调用，
    /// 让新的 g/p 关系立刻在内存图里生效，不需要重启进程。
    pub async fn reload_policy(&self) -> Result<(), CasbinError> {
        let mut enforcer = self.inner.write().await;
        enforcer
            .load_policy()
            .await
            .map_err(|e| CasbinError::PolicyOperationFailed(e.to_string()))
    }
}
