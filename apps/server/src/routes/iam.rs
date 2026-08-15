use crate::AppState;
use axum::Router;

pub fn router(app_state: &AppState) -> Router {
    let iam_state = crate::state::build_iam_state(app_state);
    iam_api::router(iam_state)
}
