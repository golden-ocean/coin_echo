use utoipa::OpenApi;

use crate::dictionary_item::{
    create::{CreateDictionaryItemReq, CreateDictionaryItemRes},
    delete::DeleteDictionaryItemRes,
    page::PageDictionaryItemRes,
    update::{UpdateDictionaryItemReq, UpdateDictionaryItemRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::dictionary_item::create::create_dictionary_item,
        crate::dictionary_item::page::page_dictionary_item,
        crate::dictionary_item::update::update_dictionary_item,
        crate::dictionary_item::delete::delete_dictionary_item,
    ),
    components(schemas(
        CreateDictionaryItemReq,
        CreateDictionaryItemRes,
        UpdateDictionaryItemReq,
        UpdateDictionaryItemRes,
        DeleteDictionaryItemRes,
        PageDictionaryItemRes,
     )),
    tags(
        (name = "SYS.DictionaryItem", description = "字典项管理"),
    ),
)]
pub struct DictionaryItemApiDoc;
