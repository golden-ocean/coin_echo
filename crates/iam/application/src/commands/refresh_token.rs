use chrono::{DateTime, Utc};

use crate::{
    error::AppError,
    ports::{PortError, TokenService, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};

pub struct RefreshTokenCommand {
    pub refresh_token: String,
}

#[derive(Debug)]
pub struct RefreshTokenResult {
    pub access_token: String,
    pub refresh_token: String,
    pub access_expires_at: DateTime<Utc>,
    pub refresh_expires_at: DateTime<Utc>,
}

/// 用 refresh token 换一对新的 access/refresh token。
///
/// 采用 refresh token 轮换（rotation）：每次刷新都签发全新的一对，旧的
/// refresh token 逻辑上应视为已消费。是否需要把旧 token 显式拉黑（依赖
/// Redis/platform-cache 维护一份撤销名单）是后续可以补充的加固点，
/// 目前签名校验层面暂不支持"提前失效"，仅靠自然过期时间兜底。
pub async fn handle_refresh_token(
    uow_factory: &dyn UnitOfWorkFactory,
    token_service: &dyn TokenService,
    cmd: RefreshTokenCommand,
) -> Result<RefreshTokenResult, AppError> {
    // 1. 先校验 refresh token 本身（签名、类型、是否过期），不涉及数据库，
    //    完全无效的 token 在这一步就能尽早失败。
    let user_id = token_service
        .verify_refresh_token(&cmd.refresh_token)
        .map_err(AppError::from)?;

    // 2. 不能相信 refresh token 里"曾经合法"这件事——账号可能在这段
    //    时间内被禁用/软删除/离职，必须重新查一次库确认当前状态。
    let user = uow_factory
        .transaction::<_, _, AppError>(|uow| {
            Box::pin(async move {
                uow.user_repo()?
                    .find_by_id(&user_id)
                    .await?
                    .ok_or(PortError::NotFound { entity: "user" })
                    .map_err(AppError::from)
            })
        })
        .await?;

    if !user.is_normal_active() {
        return Err(AppError::Forbidden);
    }

    // 3. 签发新的一对 token
    let pair = token_service
        .issue_token_pair(*user.id())
        .map_err(AppError::from)?;

    Ok(RefreshTokenResult {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
        access_expires_at: pair.access_expires_at,
        refresh_expires_at: pair.refresh_expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use chrono::{TimeZone, Utc};

    use crate::ports::{
        PermissionRepository, PortError, RolePermissionRepository, RoleRepository, TokenPair,
        TokenServiceError, UnitOfWork, UnitOfWorkError, UserRepository, UserRoleRepository,
    };
    use iam_domain::{
        id::UserId,
        user::{
            User,
            value_object::{Email, PasswordCredential, Phone, StaffNo},
        },
    };
    use platform_kernel::meta::Status;

    fn test_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn test_phc_hash() -> &'static str {
        "$argon2id$v=19,m=4096,t=3,p=2$testsalt$testhashvalue123456"
    }

    fn build_test_user(status: Status) -> User {
        let uid = UserId::generate();
        let pwd = PasswordCredential::new(test_phc_hash(), test_now()).unwrap();
        User::new(
            uid,
            "alice".to_string(),
            pwd,
            StaffNo::new("STAFF-000001").unwrap(),
            "测试用户".to_string(),
            Email::new("alice@company.com").unwrap(),
            Phone::new("13800138000").unwrap(),
            None,
            None,
            None,
            Some(status),
            test_now(),
        )
    }

    struct StubTokenService {
        verify_result: Result<UserId, TokenServiceError>,
    }
    impl TokenService for StubTokenService {
        fn issue_token_pair(&self, _user_id: UserId) -> Result<TokenPair, TokenServiceError> {
            Ok(TokenPair {
                access_token: "new-access-token".to_string(),
                refresh_token: "new-refresh-token".to_string(),
                access_expires_at: test_now() + chrono::Duration::minutes(15),
                refresh_expires_at: test_now() + chrono::Duration::days(7),
            })
        }
        fn verify_access_token(&self, _token: &str) -> Result<UserId, TokenServiceError> {
            unreachable!("刷新流程不会校验 access token")
        }
        fn verify_refresh_token(&self, _token: &str) -> Result<UserId, TokenServiceError> {
            self.verify_result.clone()
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
            Ok(self.user.as_ref().map(clone_user))
        }
        async fn find_by_username(&mut self, _u: &str) -> Result<Option<User>, PortError> {
            unreachable!()
        }
        async fn find_by_email(&mut self, _e: &Email) -> Result<Option<User>, PortError> {
            unreachable!()
        }
        async fn find_by_phone(&mut self, _p: &Phone) -> Result<Option<User>, PortError> {
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

    struct MockUnitOfWork {
        user: Option<User>,
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
            unreachable!()
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
    }
    #[async_trait]
    impl UnitOfWorkFactory for MockUnitOfWorkFactory {
        async fn begin(&self) -> Result<Box<dyn UnitOfWork>, UnitOfWorkError> {
            Ok(Box::new(MockUnitOfWork {
                user: self.user.lock().unwrap().as_ref().map(clone_user),
            }))
        }
    }

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

    #[tokio::test]
    async fn test_refresh_invalid_token_rejected() {
        let token_service = StubTokenService {
            verify_result: Err(TokenServiceError::Invalid),
        };
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(None),
        };

        let result = handle_refresh_token(
            &factory,
            &token_service,
            RefreshTokenCommand {
                refresh_token: "garbage".to_string(),
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(AppError::TokenService(TokenServiceError::Invalid))
        ));
    }

    #[tokio::test]
    async fn test_refresh_expired_token_rejected() {
        let token_service = StubTokenService {
            verify_result: Err(TokenServiceError::Expired),
        };
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(None),
        };

        let result = handle_refresh_token(
            &factory,
            &token_service,
            RefreshTokenCommand {
                refresh_token: "expired".to_string(),
            },
        )
        .await;

        assert!(matches!(
            result,
            Err(AppError::TokenService(TokenServiceError::Expired))
        ));
    }

    #[tokio::test]
    async fn test_refresh_disabled_account_rejected() {
        // token 本身合法，但账号在此期间被禁用了 —— 必须重新查库拦截，
        // 不能只信任 token 里"曾经合法"这件事。
        let user = build_test_user(Status::Disabled);
        let user_id = *user.id();
        let token_service = StubTokenService {
            verify_result: Ok(user_id),
        };
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(Some(user)),
        };

        let result = handle_refresh_token(
            &factory,
            &token_service,
            RefreshTokenCommand {
                refresh_token: "valid-but-account-disabled".to_string(),
            },
        )
        .await;

        assert!(matches!(result, Err(AppError::Forbidden)));
    }

    #[tokio::test]
    async fn test_refresh_success_issues_new_pair() {
        let user = build_test_user(Status::Enabled);
        let user_id = *user.id();
        let token_service = StubTokenService {
            verify_result: Ok(user_id),
        };
        let factory = MockUnitOfWorkFactory {
            user: Mutex::new(Some(user)),
        };

        let result = handle_refresh_token(
            &factory,
            &token_service,
            RefreshTokenCommand {
                refresh_token: "valid-token".to_string(),
            },
        )
        .await
        .expect("刷新应该成功");

        assert_eq!(result.access_token, "new-access-token");
        assert_eq!(result.refresh_token, "new-refresh-token");
    }
}
