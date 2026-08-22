use utoipa::OpenApi;

use crate::organization::{
    create::{CreateOrganizationReq, CreateOrganizationRes},
    delete::DeleteOrganizationRes,
    list::OrganizationTreeNodeRes,
    move_to::{MoveOrganizationReq, MoveOrganizationRes},
    update::{UpdateOrganizationReq, UpdateOrganizationRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::organization::create::create_organization,
        crate::organization::list::list_organization,
        crate::organization::update::update_organization,
        crate::organization::move_to::move_organization,
        crate::organization::delete::delete_organization,
    ),
    components(schemas(
        CreateOrganizationReq,
        CreateOrganizationRes,
        UpdateOrganizationReq,
        UpdateOrganizationRes,
        MoveOrganizationReq,
        MoveOrganizationRes,
        DeleteOrganizationRes,
        OrganizationTreeNodeRes,
    )),
    tags(
        (name = "ORG.Organization", description = "组织架构管理"),
    ),
)]
pub struct OrganizationApiDoc;
