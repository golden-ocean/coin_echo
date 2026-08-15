//! IAM-Role 模块的 OpenAPI 文档聚合。

use utoipa::OpenApi;

use crate::role::{
    self,
    create::{CreateRoleReq, CreateRoleRes},
    delete::DeleteRoleRes,
    page::{PageRoleReq, PageRoleRes},
    update::{UpdateRoleReq, UpdateRoleRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        role::create::create_role,
        role::page::page_role,
        role::update::update_role,
        role::delete::delete_role,
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
