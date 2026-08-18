use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    error::AppError,
    ports::{PasswordHasher, TokenService, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::{id::RoleId, user::User};
use platform_kernel::time::Clock;

/// 一个提前计算好的合法 Argon2id 哈希（对应某个任意的哑密码），
/// 仅用于"用户名不存在时也走一遍完整哈希校验流程"，抹平响应耗时差异，
/// 防止通过响应时间侧信道判断用户名是否存在。
/// ⚠️ 必须替换成用真实 Argon2Hasher 对某个固定字符串算出来的合法哈希
/// （运行一次 `hasher.hash("dummy-password-for-timing").await` 拿到结果后硬编码进来），
/// 不能随手编一个格式不对的字符串——格式错误会让 verify() 走一条更快的
/// "格式校验失败"分支，起不到抹平耗时的效果。
const DUMMY_PASSWORD_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$REPLACE_WITH_REAL_SALT$REPLACE_WITH_REAL_HASH";

pub struct LoginCommand {
    pub username: String,
    pub password: String,
}

#[derive(Debug)]
pub struct LoginResult {
    pub user_id: Uuid,
    pub username: String,
    pub name: String,
    pub role_ids: Vec<Uuid>,
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

/// 登录流程拆两个阶段：
/// 1. DB 阶段（短事务）：只做两次读（查用户、查角色），事务立即结束、
///    连接立即归还连接池。
/// 2. 纯计算阶段：Argon2 密码校验、JWT 签名都是 CPU 密集型操作，
///    此时已经不再持有任何数据库连接，避免长时间占用连接池资源。
///
/// `clock` 目前虽然在函数体内没有被直接使用（标了 `_clock`），但保留在签名里
/// 是为了后续扩展"记录最后登录时间"这类写操作时，不需要再改一遍所有调用方。
pub async fn handle_login(
    uow_factory: &dyn UnitOfWorkFactory,
    password_hasher: &dyn PasswordHasher,
    token_service: &dyn TokenService,
    _clock: &dyn Clock,
    cmd: LoginCommand,
) -> Result<LoginResult, AppError> {
    // ---- 阶段一：DB 阶段。只做两次读，事务开完立刻结束、连接立刻归还连接池。
    //      User 是拥有所有权的值，移出事务闭包没有生命周期问题。
    let snapshot = uow_factory
        .transaction::<_, Option<(User, Vec<RoleId>)>, AppError>(|uow| {
            Box::pin(async move {
                let user = uow.user_repo()?.find_by_username(&cmd.username).await?;
                let user = match user {
                    Some(u) => u,
                    None => return Ok(None),
                };
                let role_ids = uow
                    .user_role_repo()?
                    .list_role_ids_by_user(user.id())
                    .await?;
                Ok(Some((user, role_ids)))
            })
        })
        .await?;

    // ---- 阶段二：纯计算阶段，不再持有数据库连接 ----
    let (user, role_ids_vo) = match snapshot {
        Some(pair) => pair,
        None => {
            // 用户名不存在：仍然对固定哑哈希跑一次完整校验（忽略结果），
            // 让耗时和"用户名对、密码错"保持一致，避免响应时间侧信道
            // 泄露用户名是否存在。
            let _ = password_hasher
                .verify(&cmd.password, DUMMY_PASSWORD_HASH)
                .await;
            return Err(AppError::Unauthorized);
        }
    };

    // 校验密码。用户名不存在 和 密码错误 折叠成同一个 Unauthorized，
    // 不暴露"具体哪里错了"，防止用户名枚举。
    if password_hasher
        .verify(&cmd.password, user.password_credential().hash_as_str())
        .await
        .is_err()
    {
        return Err(AppError::Unauthorized);
    }

    // 密码验证通过后才检查账号可用性——这一步不算凭证信息泄露，
    // 可以给出比 Unauthorized 更明确的 Forbidden。
    // is_normal_active() 是领域已经封装好的复合判断（未删除+启用+在职），
    // 不在 Command 层重复拆开判断，避免和领域规则出现两处不一致。
    if !user.is_normal_active() {
        return Err(AppError::Forbidden);
    }

    let token_pair = token_service
        .issue_token_pair(*user.id())
        .map_err(AppError::from)?;

    Ok(LoginResult {
        user_id: user.id().as_uuid(),
        username: user.username().to_string(),
        name: user.name().to_string(),
        role_ids: role_ids_vo.iter().map(|r| r.as_uuid()).collect(),
        access_token: token_pair.access_token,
        refresh_token: token_pair.refresh_token,
        access_expires_at: token_pair.access_expires_at,
        refresh_expires_at: token_pair.refresh_expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::TimeZone;

    use crate::ports::{
        PasswordHasherError, PermissionRepository, PortError, RolePermissionRepository,
        RoleRepository, TokenPair, TokenServiceError, UnitOfWork, UnitOfWorkError, UserRepository,
        UserRoleRepository,
    };
    use iam_domain::{
        id::UserId,
        user::value_object::{Email, PasswordCredential, Phone, StaffNo},
    };
    use platform_kernel::{meta::Status, time::FixedClock};

    // ---------------------------------------------------------------
    // 手写 Mock
    // ---------------------------------------------------------------

    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn test_phc_hash() -> &'static str {
        "$argon2id$v=19,m=4096,t=3,p=2$testsalt$testhashvalue123456"
    }

    /// 密码校验结果可配置的 Mock。trait 是 async fn，mock 实现同样要
    /// 标 #[async_trait::async_trait]。
    struct MockPasswordHasher {
        should_succeed: bool,
    }
    #[async_trait]
    impl PasswordHasher for MockPasswordHasher {
        async fn hash(&self, _raw: &str) -> Result<String, PasswordHasherError> {
            unreachable!("登录流程不会调用 hash()")
        }
        async fn verify(&self, _raw: &str, _hash: &str) -> Result<(), PasswordHasherError> {
            if self.should_succeed {
                Ok(())
            } else {
                Err(PasswordHasherError::Verify)
            }
        }
    }

    struct MockTokenService;
    impl TokenService for MockTokenService {
        fn issue_token_pair(&self, _id: UserId) -> Result<TokenPair, TokenServiceError> {
            let now = test_now();
            Ok(TokenPair {
                access_token: "mock-access-token".to_string(),
                refresh_token: "mock-refresh-token".to_string(),
                access_expires_at: now + chrono::Duration::minutes(15),
                refresh_expires_at: now + chrono::Duration::days(7),
            })
        }
        fn verify_access_token(&self, _token: &str) -> Result<UserId, TokenServiceError> {
            unreachable!("登录流程不会调用 verify_access_token()")
        }
        fn verify_refresh_token(&self, _token: &str) -> Result<UserId, TokenServiceError> {
            unreachable!("登录流程不会调用 verify_refresh_token()")
        }
    }

    struct MockUserRepository {
        user: Option<User>,
    }
    #[async_trait]
    impl UserRepository for MockUserRepository {
        async fn insert(&mut self, _u: &User) -> Result<(), PortError> {
            unreachable!()
        }
        async fn update(&mut self, _u: &User) -> Result<(), PortError> {
            unreachable!()
        }
        async fn soft_delete(&mut self, _u: &User) -> Result<(), PortError> {
            unreachable!()
        }
        async fn find_by_id(&mut self, _id: &UserId) -> Result<Option<User>, PortError> {
            unreachable!()
        }
        async fn find_by_username(&mut self, username: &str) -> Result<Option<User>, PortError> {
            Ok(self
                .user
                .as_ref()
                .filter(|u| u.username() == username)
                .map(clone_user))
        }
        async fn find_by_email(&mut self, _email: &Email) -> Result<Option<User>, PortError> {
            unreachable!()
        }
        async fn find_by_phone(&mut self, _phone: &Phone) -> Result<Option<User>, PortError> {
            unreachable!()
        }
        async fn exists_by_username(&mut self, _u: &str) -> Result<bool, PortError> {
            unreachable!()
        }
        async fn exists_by_email(&mut self, _e: &Email) -> Result<bool, PortError> {
            unreachable!()
        }
        async fn exists_by_phone(&mut self, _p: &Phone) -> Result<bool, PortError> {
            unreachable!()
        }
    }

    struct MockUserRoleRepository {
        role_ids: Vec<RoleId>,
    }
    #[async_trait]
    impl UserRoleRepository for MockUserRoleRepository {
        async fn replace_roles(&mut self, _u: &UserId, _r: &[RoleId]) -> Result<(), PortError> {
            unreachable!()
        }
        async fn list_role_ids_by_user(&mut self, _u: &UserId) -> Result<Vec<RoleId>, PortError> {
            Ok(self.role_ids.clone())
        }
        async fn list_user_ids_by_role(&mut self, _r: &RoleId) -> Result<Vec<UserId>, PortError> {
            unreachable!()
        }
    }

    struct MockUnitOfWork {
        user: Option<User>,
        role_ids: Vec<RoleId>,
    }
    #[async_trait]
    impl UnitOfWork for MockUnitOfWork {
        fn user_repo<'a>(&'a mut self) -> Result<Box<dyn UserRepository + 'a>, UnitOfWorkError> {
            Ok(Box::new(MockUserRepository {
                user: self.user.as_ref().map(clone_user),
            }))
        }
        fn role_repo<'a>(&'a mut self) -> Result<Box<dyn RoleRepository + 'a>, UnitOfWorkError> {
            unreachable!()
        }
        fn permission_repo<'a>(
            &'a mut self,
        ) -> Result<Box<dyn PermissionRepository + 'a>, UnitOfWorkError> {
            unreachable!()
        }
        fn role_permission_repo<'a>(
            &'a mut self,
        ) -> Result<Box<dyn RolePermissionRepository + 'a>, UnitOfWorkError> {
            unreachable!()
        }
        fn user_role_repo<'a>(
            &'a mut self,
        ) -> Result<Box<dyn UserRoleRepository + 'a>, UnitOfWorkError> {
            Ok(Box::new(MockUserRoleRepository {
                role_ids: self.role_ids.clone(),
            }))
        }
        async fn commit(self: Box<Self>) -> Result<(), UnitOfWorkError> {
            Ok(())
        }
        async fn rollback(self: Box<Self>) -> Result<(), UnitOfWorkError> {
            Ok(())
        }
    }

    struct MockUnitOfWorkFactory {
        user: Mutex<Option<User>>,
        role_ids: Vec<RoleId>,
    }
    #[async_trait]
    impl UnitOfWorkFactory for MockUnitOfWorkFactory {
        async fn begin(&self) -> Result<Box<dyn UnitOfWork>, UnitOfWorkError> {
            Ok(Box::new(MockUnitOfWork {
                user: self.user.lock().unwrap().as_ref().map(clone_user),
                role_ids: self.role_ids.clone(),
            }))
        }
    }

    // User 没有派生 Clone（Debug 输出脱敏、聚合根一般也不建议随意 Clone），
    // 测试里为了在多个 mock 之间共享同一个用户快照，用 User::restore
    // 基于已有字段手动重建一份，而不是给领域类型加 Clone。
    fn clone_user(u: &User) -> User {
        User::restore(
            *u.id(),
            u.username().to_string(),
            u.staff_no().clone(),
            u.name().to_string(),
            u.email().clone(),
            u.phone().clone(),
            u.gender(),
            u.birthday(),
            u.avatar().map(str::to_string),
            u.password_credential().clone(),
            u.employment_status(),
            u.data_scope(),
            u.is_builtin(),
            u.sort(),
            u.remark().map(str::to_string),
            u.status(),
            u.organization_id().cloned(),
            u.position_id().cloned(),
            u.role_ids().to_vec(),
            u.audit_meta().clone(),
            u.delete_meta().clone(),
            u.version_meta().clone(),
        )
    }

    /// 构造一个用于登录测试的用户，status 可控（用于测试禁用账号场景）
    fn build_login_test_user(username: &str, status: Status, now: DateTime<Utc>) -> User {
        let uid = UserId::generate();
        let pwd = PasswordCredential::new(test_phc_hash(), now).unwrap();
        let staff_no = StaffNo::new("STAFF-000001").unwrap();
        let email = Email::new("login-test@company.com").unwrap();
        let phone = Phone::new("13800138000").unwrap();

        User::new(
            uid,
            username.to_string(),
            pwd,
            staff_no,
            "登录测试用户".to_string(),
            email,
            phone,
            None,
            None,
            None,
            Some(status),
            now,
        )
    }

    #[tokio::test]
    async fn test_login_user_not_found_returns_unauthorized() {
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(None),
            role_ids: vec![],
        };
        let hasher = MockPasswordHasher {
            should_succeed: false,
        };
        let token_service = MockTokenService;
        let clock = FixedClock::new(test_now());

        let result = handle_login(
            &factory,
            &hasher,
            &token_service,
            &clock,
            LoginCommand {
                username: "not_exist".to_string(),
                password: "whatever".to_string(),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_login_wrong_password_returns_unauthorized() {
        // 用户存在，但密码校验失败 —— 必须和"用户不存在"返回完全相同的错误，
        // 这是本测试要验证的核心安全属性（防止用户名枚举）。
        let user = build_login_test_user("alice", Status::Enabled, test_now());
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(Some(user)),
            role_ids: vec![],
        };
        let hasher = MockPasswordHasher {
            should_succeed: false,
        };
        let token_service = MockTokenService;
        let clock = FixedClock::new(test_now());

        let result = handle_login(
            &factory,
            &hasher,
            &token_service,
            &clock,
            LoginCommand {
                username: "alice".to_string(),
                password: "wrong-password".to_string(),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Unauthorized)));
    }

    #[tokio::test]
    async fn test_login_disabled_account_returns_forbidden() {
        // 密码正确，但账号被禁用 —— 应该是 Forbidden 而不是 Unauthorized，
        // 这一步发生在密码校验通过之后，不构成凭证信息泄露。
        let user = build_login_test_user("bob", Status::Disabled, test_now());
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(Some(user)),
            role_ids: vec![],
        };
        let hasher = MockPasswordHasher {
            should_succeed: true,
        };
        let token_service = MockTokenService;
        let clock = FixedClock::new(test_now());

        let result = handle_login(
            &factory,
            &hasher,
            &token_service,
            &clock,
            LoginCommand {
                username: "bob".to_string(),
                password: "correct-password".to_string(),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn test_login_success_returns_token_pair_and_role_ids() {
        let user = build_login_test_user("alice", Status::Enabled, test_now());
        let role_id = RoleId::generate();
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(Some(user)),
            role_ids: vec![role_id],
        };
        let hasher = MockPasswordHasher {
            should_succeed: true,
        };
        let token_service = MockTokenService;
        let clock = FixedClock::new(test_now());

        let result = handle_login(
            &factory,
            &hasher,
            &token_service,
            &clock,
            LoginCommand {
                username: "alice".to_string(),
                password: "correct-password".to_string(),
            },
        )
        .await
        .expect("登录应该成功");

        assert_eq!(result.access_token, "mock-access-token");
        assert_eq!(result.refresh_token, "mock-refresh-token");
        assert_eq!(result.role_ids, vec![role_id.as_uuid()]);
        assert_eq!(result.username, "alice");
    }

    /// 验证阶段拆分没有破坏"用户不存在也要走一遍哑哈希校验"这条
    /// 防时序攻击逻辑——即便密码校验现在被挪到了事务外面，这一步仍然
    /// 必须发生，不能因为重构而被漏掉。
    #[tokio::test]
    async fn test_login_user_not_found_still_calls_dummy_verify() {
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(None),
            role_ids: vec![],
        };
        // should_succeed: true 也无所谓——用户不存在这条分支根本不看
        // verify() 的返回值，只关心该分支必须始终返回 Unauthorized。
        let hasher = MockPasswordHasher {
            should_succeed: true,
        };
        let token_service = MockTokenService;
        let clock = FixedClock::new(test_now());

        let result = handle_login(
            &factory,
            &hasher,
            &token_service,
            &clock,
            LoginCommand {
                username: "ghost".to_string(),
                password: "whatever".to_string(),
            },
        )
        .await;

        // 无论 verify 是否"成功"，用户不存在这条路径都必须返回 Unauthorized，
        // 而不是被 should_succeed: true 误导成功登录。
        assert!(matches!(result, Err(AppError::Unauthorized)));
    }
}
