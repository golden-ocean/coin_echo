mod assign_permissions;
mod create;
mod delete;
mod openapi;
mod page;
mod permissions;
mod update;

pub use assign_permissions::assign_role_permissions;
pub use create::create_role;
pub use delete::delete_role;
pub use openapi::RoleApiDoc;
pub use page::page_role;
pub use permissions::get_role_permissions;
pub use update::update_role;
