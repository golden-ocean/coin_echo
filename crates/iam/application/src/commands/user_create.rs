use crate::{
    error::AppError,
    ports::{PasswordHasher, PortError, StaffNoGenerator, UnitOfWorkFactory, UnitOfWorkFactoryExt},
};
use iam_domain::{
    error::DomainError,
    id::{OrganizationId, UserId},
    user::{
        User,
        value_object::{Email, PasswordCredential, Phone},
    },
};
use platform_kernel::{meta::Status, time::Clock};
use uuid::Uuid;

pub struct UserCreateCommand {
    pub username: String,
    pub password: String,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub organization_id: Option<Uuid>,
    pub sort: Option<i32>,
    pub status: Option<Status>,
    pub operator_id: Option<Uuid>,
}

pub async fn handle_user_create(
    uow_factory: &dyn UnitOfWorkFactory,
    password_hasher: &dyn PasswordHasher,
    staff_no_generator: &dyn StaffNoGenerator,
    clock: &dyn Clock,
    cmd: UserCreateCommand,
) -> Result<(), AppError> {
    let now = clock.now();
    let email_vo = Email::new(&cmd.email).map_err(DomainError::from)?;
    let phone_vo = Phone::new(&cmd.phone).map_err(DomainError::from)?;
    let org_id_vo = cmd.organization_id.map(OrganizationId::from);
    let password_hash = password_hasher.hash(&cmd.password).await?;
    let password_credential_vo =
        PasswordCredential::new(&password_hash, now).map_err(DomainError::from)?;
    let new_user_id = UserId::generate();
    let operator_id_vo = cmd.operator_id;

    // 挪到事务外：nextval() 是非事务性的，不需要（也不应该）绑定在应用层事务里
    let staff_no_vo = staff_no_generator.generate().await?;

    uow_factory
        .transaction::<_, (), AppError>(|uow| {
            Box::pin(async move {
                if uow.user_repo()?.exists_by_username(&cmd.username).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "user",
                        field: "username",
                    }));
                }
                if uow.user_repo()?.exists_by_email(&email_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "user",
                        field: "email",
                    }));
                }
                if uow.user_repo()?.exists_by_phone(&phone_vo).await? {
                    return Err(AppError::from(PortError::UniqueConflict {
                        entity: "user",
                        field: "phone",
                    }));
                }

                let new_user = User::new(
                    new_user_id,
                    cmd.username,
                    password_credential_vo,
                    staff_no_vo,
                    cmd.name,
                    email_vo,
                    phone_vo,
                    org_id_vo,
                    operator_id_vo,
                    cmd.sort,
                    cmd.status,
                    now,
                );
                uow.user_repo()?.insert(&new_user).await?;
                Ok(())
            })
        })
        .await
}
