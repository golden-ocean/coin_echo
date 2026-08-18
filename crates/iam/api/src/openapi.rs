use utoipa::OpenApi;

use crate::auth::AuthApiDoc;
use crate::permission::PermissionApiDoc;
use crate::role::RoleApiDoc;
use crate::user::UserApiDoc;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/roles", api = RoleApiDoc),
        (path = "/users", api = UserApiDoc),
        (path = "/permissions", api = PermissionApiDoc),
        (path = "/auth", api = AuthApiDoc),
    ),
)]
pub struct IamApiDoc;
