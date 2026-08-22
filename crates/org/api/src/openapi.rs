use utoipa::OpenApi;

use crate::organization::OrganizationApiDoc;
use crate::position::PositionApiDoc;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/organizations", api = OrganizationApiDoc),
        (path = "/positions", api = PositionApiDoc),
    ),
)]
pub struct OrgApiDoc;
