use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/roles", api = crate::role::RoleApiDoc),
        (path = "/users", api = crate::user::UserApiDoc),
        (path = "/permissions", api = crate::permission::PermissionApiDoc),
        (path = "/auth", api = crate::auth::AuthApiDoc),
    ),
)]
pub struct IamApiDoc;
