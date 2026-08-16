use utoipa::OpenApi;

use crate::permission::PermissionApiDoc;
use crate::role::RoleApiDoc;
use crate::user::UserApiDoc;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/roles", api = RoleApiDoc),
        (path = "/users", api = UserApiDoc),
        (path = "/permissions", api = PermissionApiDoc),
    ),
)]
pub struct IamApiDoc;
