use platform_kernel::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DictionaryItemMarker;

pub type DictionaryId = Id<DictionaryMarker>;
pub type DictionaryItemId = Id<DictionaryItemMarker>;
