use utoipa::OpenApi;

use crate::position::{
    create::{CreatePositionReq, CreatePositionRes},
    delete::DeletePositionRes,
    list::PositionListItemRes,
    update::{UpdatePositionReq, UpdatePositionRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::position::create::create_position,
        crate::position::list::list_position,
        crate::position::update::update_position,
        crate::position::delete::delete_position,
    ),
    components(schemas(
        CreatePositionReq,
        CreatePositionRes,
        UpdatePositionReq,
        UpdatePositionRes,
        DeletePositionRes,
        PositionListItemRes,
    )),
    tags(
        (name = "ORG.Position", description = "职位管理"),
    ),
)]
pub struct PositionApiDoc;
