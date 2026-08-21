use utoipa::OpenApi;

use crate::permission::{
    create::{CreatePermissionReq, CreatePermissionRes},
    delete::DeletePermissionRes,
    list::{ListPermissionReq, ListPermissionRes},
    update::{UpdatePermissionReq, UpdatePermissionRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::permission::create::create_permission,
        crate::permission::list::list_permission,
        crate::permission::update::update_permission,
        crate::permission::delete::delete_permission,
    ),
    components(schemas(
        CreatePermissionReq,
        CreatePermissionRes,
        UpdatePermissionReq,
        UpdatePermissionRes,
        ListPermissionReq,
        ListPermissionRes,
        DeletePermissionRes,
    )),
    tags(
        (name = "IAM.Permission", description = "权限管理"),
    ),
)]
pub struct PermissionApiDoc;
