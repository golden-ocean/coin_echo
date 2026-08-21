use utoipa::OpenApi;

use crate::dictionary::{
    create::{CreateDictionaryReq, CreateDictionaryRes},
    delete::DeleteDictionaryRes,
    list::ListDictionaryRes,
    update::{UpdateDictionaryReq, UpdateDictionaryRes},
};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::dictionary::create::create_dictionary,
        crate::dictionary::list::list_dictionary,
        crate::dictionary::update::update_dictionary,
        crate::dictionary::delete::delete_dictionary,
    ),
    components(schemas(
        CreateDictionaryReq,
        CreateDictionaryRes,
        UpdateDictionaryReq,
        UpdateDictionaryRes,
        DeleteDictionaryRes,
        ListDictionaryRes,
     )),
    tags(
        (name = "SYS.Dictionary", description = "字典管理"),
    ),
)]
pub struct DictionaryApiDoc;
