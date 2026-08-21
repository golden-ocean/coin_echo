mod create;
mod delete;
mod list;
mod openapi;
mod update;

pub use create::create_dictionary;
pub use delete::delete_dictionary;
pub use list::list_dictionary;
pub use openapi::DictionaryApiDoc;
pub use update::update_dictionary;
