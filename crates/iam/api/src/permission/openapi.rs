//! IAM-Permission 模块的 OpenAPI 文档聚合。
use utoipa::OpenApi;

use crate::permission::{
    self,
    create::{CreatePermissionReq, CreatePermissionRes},
    delete::DeletePermissionRes,
    list::{ListPermissionReq, ListPermissionRes},
    update::{UpdatePermissionReq, UpdatePermissionRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        permission::create::create_permission,
        permission::list::list_permission,
        permission::update::update_permission,
        permission::delete::delete_permission,
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

