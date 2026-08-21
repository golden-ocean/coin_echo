use utoipa::OpenApi;

use crate::user::{
    create::{CreateUserReq, CreateUserRes},
    delete::DeleteUserRes,
    page::{PageUserReq, PageUserRes},
    update::{UpdateUserReq, UpdateUserRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::user::create::create_user,
        crate::user::page::page_user,
        crate::user::update::update_user,
        crate::user::delete::delete_user,
        crate::user::roles::list_user_roles,
    ),
    components(schemas(
         CreateUserReq,
         CreateUserRes,
         UpdateUserReq,
         UpdateUserRes,
         PageUserReq,
         PageUserRes,
         DeleteUserRes,
     )),
    tags(
        (name = "IAM.User", description = "用户管理"),
    ),
)]
pub struct UserApiDoc;
