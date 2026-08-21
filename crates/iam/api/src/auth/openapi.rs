use utoipa::OpenApi;

use crate::auth::{
    login::{LoginReq, LoginRes},
    refresh::{RefreshTokenReq, RefreshTokenRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::auth::login::login,
        crate::auth::refresh::refresh_token,
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
