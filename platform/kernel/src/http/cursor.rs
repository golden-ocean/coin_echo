//! 游标分页契约。

/// 游标。
///
/// 内容对客户端**不透明**：其编码方式（排序键 + 主键的组合）属于服务端实现
/// 细节，客户端只需原样回传。若把可解析的结构暴露出去，客户端一定会开始构造
/// 游标，服务端排序策略就再也改不动了。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Cursor(String);

impl Cursor {
    /// 由编码后的字符串构造。
    #[must_use]
    pub const fn new(encoded: String) -> Self {
        Self(encoded)
    }

    /// 底层字符串表示（借用）。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 取出底层字符串（消费 `self`）。
    ///
    /// 用于游标需要被移动进下一层调用（如拼接查询参数）而非仅借用的场景，
    /// 避免多一次 `to_string()` 分配。
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// 游标分页请求。
///
/// 字段私有并通过取值方法暴露：与 `PageQuery` 同样的理由——保证任何来源
/// 的输入都经过 clamp，不存在绕过上限的路径。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CursorPaginationParams {
    /// 上一页返回的游标；首页为 `None`。
    #[serde(default)]
    after: Option<Cursor>,
    #[serde(default = "CursorPaginationParams::default_limit")]
    limit: u32,
}

impl CursorPaginationParams {
    /// 单次返回条数上限。超过部分被截断而非报错，理由同 [`PageQuery::MAX_PER_PAGE`]。
    pub const MAX_LIMIT: u32 = 200;
    /// 未指定时的返回条数。
    pub const DEFAULT_LIMIT: u32 = 20;

    const fn default_limit() -> u32 {
        Self::DEFAULT_LIMIT
    }

    /// 构造游标分页请求。取值在读取时统一 clamp，此处无需预处理。
    #[must_use]
    pub const fn new(after: Option<Cursor>, limit: u32) -> Self {
        Self { after, limit }
    }

    /// 上一页返回的游标；首页为 `None`。
    #[must_use]
    pub const fn after(&self) -> Option<&Cursor> {
        self.after.as_ref()
    }

    /// 单次返回条数，限定在 `1..=MAX_LIMIT`。
    #[must_use]
    pub const fn limit(&self) -> u32 {
        if self.limit == 0 {
            Self::DEFAULT_LIMIT
        } else if self.limit > Self::MAX_LIMIT {
            Self::MAX_LIMIT
        } else {
            self.limit
        }
    }
}

impl Default for CursorPaginationParams {
    fn default() -> Self {
        Self {
            after: None,
            limit: Self::DEFAULT_LIMIT,
        }
    }
}

/// 游标分页结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CursorPaginatedResponse<T> {
    /// 本批数据。
    pub items: Vec<T>,
    /// 下一页游标。为 `None` 表示已到末尾。
    ///
    /// 客户端应据此判断是否继续拉取，而**不是**判断 `items` 是否少于 `limit`
    /// —— 存在过滤条件时，某一批可能返回不足 `limit` 条但后面仍有数据。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,
}

impl<T> CursorPaginatedResponse<T> {
    /// 构造游标分页结果。
    #[must_use]
    pub const fn new(items: Vec<T>, next_cursor: Option<Cursor>) -> Self {
        Self { items, next_cursor }
    }

    /// 逐元素转换。
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> CursorPaginatedResponse<U> {
        CursorPaginatedResponse {
            items: self.items.into_iter().map(f).collect(),
            next_cursor: self.next_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Cursor ----

    #[test]
    fn cursor_as_str_borrows_without_consuming() {
        let cursor = Cursor::new("eyJpZCI6MTIzfQ".to_string());
        assert_eq!(cursor.as_str(), "eyJpZCI6MTIzfQ");
        // as_str 只借用，cursor 之后仍可使用
        assert_eq!(cursor.as_str(), "eyJpZCI6MTIzfQ");
    }

    #[test]
    fn cursor_into_inner_consumes_and_returns_owned_string() {
        let cursor = Cursor::new("eyJpZCI6MTIzfQ".to_string());
        let raw: String = cursor.into_inner();
        assert_eq!(raw, "eyJpZCI6MTIzfQ");
    }

    #[test]
    fn cursor_serializes_as_bare_string_not_wrapped_object() {
        // #[serde(transparent)] 的关键行为：序列化结果应是裸字符串，
        // 而不是 {"0": "..."} —— 否则客户端原样回传时结构就不匹配了。
        let cursor = Cursor::new("abc123".to_string());
        let json = serde_json::to_string(&cursor).unwrap();
        assert_eq!(json, "\"abc123\"");
    }

    #[test]
    fn cursor_deserializes_from_bare_string() {
        let cursor: Cursor = serde_json::from_str("\"abc123\"").unwrap();
        assert_eq!(cursor.as_str(), "abc123");
    }

    #[test]
    fn cursor_roundtrips_through_json() {
        let original = Cursor::new("opaque-token-xyz".to_string());
        let json = serde_json::to_string(&original).unwrap();
        let restored: Cursor = serde_json::from_str(&json).unwrap();
        assert_eq!(original, restored);
    }

    // ---- CursorPaginationParams::limit clamp ----

    #[test]
    fn limit_zero_falls_back_to_default() {
        let query = CursorPaginationParams::new(None, 0);
        assert_eq!(query.limit(), CursorPaginationParams::DEFAULT_LIMIT);
    }

    #[test]
    fn limit_within_range_is_unchanged() {
        let query = CursorPaginationParams::new(None, 50);
        assert_eq!(query.limit(), 50);
    }

    #[test]
    fn limit_exceeding_max_is_clamped() {
        let query = CursorPaginationParams::new(None, 999_999);
        assert_eq!(query.limit(), CursorPaginationParams::MAX_LIMIT);
    }

    #[test]
    fn limit_exactly_at_max_is_unchanged() {
        let query = CursorPaginationParams::new(None, CursorPaginationParams::MAX_LIMIT);
        assert_eq!(query.limit(), CursorPaginationParams::MAX_LIMIT);
    }

    // ---- CursorPaginationParams::after ----

    #[test]
    fn after_returns_none_for_first_page() {
        let query = CursorPaginationParams::new(None, 20);
        assert!(query.after().is_none());
    }

    #[test]
    fn after_returns_the_given_cursor() {
        let cursor = Cursor::new("token-1".to_string());
        let query = CursorPaginationParams::new(Some(cursor.clone()), 20);
        assert_eq!(query.after(), Some(&cursor));
    }

    // ---- CursorPaginationParams::default ----

    #[test]
    fn default_has_no_after_and_default_limit() {
        let query = CursorPaginationParams::default();
        assert!(query.after().is_none());
        assert_eq!(query.limit(), CursorPaginationParams::DEFAULT_LIMIT);
    }

    // ---- CursorPaginationParams deserialize (missing fields use #[serde(default)]) ----

    #[test]
    fn deserializes_from_empty_json_object_using_defaults() {
        let query: CursorPaginationParams = serde_json::from_str("{}").unwrap();
        assert!(query.after().is_none());
        assert_eq!(query.limit(), CursorPaginationParams::DEFAULT_LIMIT);
    }

    #[test]
    fn deserializes_with_after_and_limit_present() {
        let json = r#"{"after":"token-abc","limit":30}"#;
        let query: CursorPaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(query.after().map(Cursor::as_str), Some("token-abc"));
        assert_eq!(query.limit(), 30);
    }

    #[test]
    fn deserialize_still_clamps_oversized_limit() {
        // 反序列化本身不报错（u32 足够宽），clamp 发生在取值时。
        let json = r#"{"limit":100000}"#;
        let query: CursorPaginationParams = serde_json::from_str(json).unwrap();
        assert_eq!(query.limit(), CursorPaginationParams::MAX_LIMIT);
    }

    // ---- CursorPaginatedResponse ----

    #[test]
    fn cursor_page_new_stores_items_and_next_cursor() {
        let page =
            CursorPaginatedResponse::new(vec![1, 2, 3], Some(Cursor::new("next".to_string())));
        assert_eq!(page.items, vec![1, 2, 3]);
        assert_eq!(page.next_cursor.as_ref().map(Cursor::as_str), Some("next"));
    }

    #[test]
    fn cursor_page_next_cursor_none_means_no_more_pages() {
        let page: CursorPaginatedResponse<i32> = CursorPaginatedResponse::new(vec![1, 2], None);
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn cursor_page_map_transforms_items_and_preserves_cursor() {
        let page =
            CursorPaginatedResponse::new(vec![1, 2, 3], Some(Cursor::new("next".to_string())));
        let mapped = page.map(|n| n.to_string());
        assert_eq!(
            mapped.items,
            vec!["1".to_string(), "2".to_string(), "3".to_string()]
        );
        assert_eq!(
            mapped.next_cursor.as_ref().map(Cursor::as_str),
            Some("next")
        );
    }

    #[test]
    fn cursor_page_next_cursor_omitted_from_json_when_none() {
        // #[serde(skip_serializing_if = "Option::is_none")] 的行为验证：
        // 到达末页时响应体里不应出现多余的 null 字段。
        let page: CursorPaginatedResponse<i32> = CursorPaginatedResponse::new(vec![1], None);
        let json = serde_json::to_string(&page).unwrap();
        assert!(!json.contains("next_cursor"));
    }

    #[test]
    fn cursor_page_next_cursor_included_when_present() {
        let page = CursorPaginatedResponse::new(vec![1], Some(Cursor::new("tok".to_string())));
        let json = serde_json::to_string(&page).unwrap();
        assert!(json.contains("\"next_cursor\":\"tok\""));
    }
}
