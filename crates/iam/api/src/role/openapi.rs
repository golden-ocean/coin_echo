use utoipa::OpenApi;

use crate::role::{
    create::{CreateRoleReq, CreateRoleRes},
    delete::DeleteRoleRes,
    page::{PageRoleReq, PageRoleRes},
    update::{UpdateRoleReq, UpdateRoleRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::role::create::create_role,
        crate::role::page::page_role,
        crate::role::update::update_role,
        crate::role::delete::delete_role,
        crate::role::permissions::list_role_permissions,
    ),
    components(schemas(
         CreateRoleReq,
         CreateRoleRes,
         UpdateRoleReq,
         UpdateRoleRes,
         PageRoleReq,
         PageRoleRes,
         DeleteRoleRes,
     )),
    tags(
        (name = "IAM.Role", description = "角色管理"),
    ),
)]
pub struct RoleApiDoc;
