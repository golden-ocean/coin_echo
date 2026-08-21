mod error;

mod dict_item_page;
mod dict_list;

pub use dict_item_page::{
    DictionaryItemPageItem, DictionaryItemPageQuery, handle_dictionary_item_page,
};
pub use dict_list::{DictionaryListItem, handle_dictionary_list};
pub use error::QueryError;
