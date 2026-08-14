use utoipa::OpenApi;

use crate::role::RoleApiDoc;
use crate::user::UserApiDoc;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/roles", api = RoleApiDoc),
        (path = "/users", api = UserApiDoc),
    ),
)]
pub struct IamApiDoc;
