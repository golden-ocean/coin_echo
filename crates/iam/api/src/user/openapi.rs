//! IAM-User 模块的 OpenAPI 文档聚合。

use utoipa::OpenApi;

use crate::user::{
    self,
    create::{CreateReq, CreateRes},
    delete::DeleteRes,
    page::PageRes,
    update::{UpdateReq, UpdateRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        user::create::create_user,
        user::page::page_user,
        user::update::update_user,
        user::delete::delete_user,
    ),
    components(schemas(
         CreateReq,
         CreateRes,
         UpdateReq,
         UpdateRes,
         PageRes,
         DeleteRes,
     )),
    tags(
        (name = "IAM.User", description = "用户管理"),
    ),
)]
pub struct UserApiDoc;
