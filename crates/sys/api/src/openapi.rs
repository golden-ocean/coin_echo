use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    nest(
        (path = "/dictionaries", api = crate::dictionary::DictionaryApiDoc),
        (path = "/dictionary/items", api = crate::dictionary_item::DictionaryItemApiDoc),
    ),
)]
pub struct SysApiDoc;
