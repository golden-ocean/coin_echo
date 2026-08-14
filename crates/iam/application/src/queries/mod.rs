pub mod error;
pub mod role_page;
pub mod user_page;

pub use error::QueryError;
pub use role_page::{RolePageQuery, handle_role_page};
pub use user_page::{UserPageQuery, handle_user_page};
