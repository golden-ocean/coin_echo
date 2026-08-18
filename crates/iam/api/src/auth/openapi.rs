//! IAM-Auth 模块的 OpenAPI 文档聚合。
use utoipa::OpenApi;

use crate::auth::{
    self,
    login::{LoginReq, LoginRes},
    refresh::{RefreshTokenReq, RefreshTokenRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::login::login,
        auth::refresh::refresh_token,
    ),
    components(schemas(
        LoginReq,
        LoginRes,
        RefreshTokenReq,
        RefreshTokenRes,
    )),
    tags(
        (name = "IAM.Auth", description = "认证与令牌管理"),
    ),
)]
pub struct AuthApiDoc;
