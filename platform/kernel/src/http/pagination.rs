//! 分页契约。
//!
//! 同时提供两种分页模型，按场景选用：
//!
//! | | [`PageQuery`]（偏移） | [`CursorQuery`]（游标） |
//! |---|---|---|
//! | 跳页 | 支持 | 不支持 |
//! | 总数 | 可给出 | 不给出 |
//! | 深翻性能 | `OFFSET` 越大越慢 | 恒定 |
//! | 数据变动时 | 可能重复/漏记录 | 稳定 |
//!
//! **管理后台列表用偏移分页**（用户需要跳页和总数），
//! **面向终端用户的信息流用游标分页**（数据持续插入，偏移分页会让用户反复看到同一条）。
//!
//! # 两条硬性约束
//!
//! 1. **每页大小必须有上限**。不设上限时 `?per_page=1000000` 就是一个免费的
//!    内存放大攻击入口。这里在取值处强制 clamp，任何来源的输入都无法绕过。
//! 2. **总数按需计算**。`COUNT(*)` 在大表上是全扫描，默认返回会让每次列表查询
//!    的成本翻倍，因此 [`Page::total`] 是 `Option`。

/// 偏移分页请求。
///
/// 字段私有并通过取值方法暴露：这样无论来自查询串、JSON 还是代码直接构造，
/// 都保证经过 clamp，不存在绕过上限的路径。
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct PaginationQuery {
    #[serde(default = "PaginationQuery::default_page")]
    page: u32,
    #[serde(default = "PaginationQuery::default_per_page", rename = "per_page")]
    per_page: u32,
}

impl PaginationQuery {
    /// 每页条数上限。超过部分被截断而非报错 —— 对列表接口而言，返回上限条数
    /// 比抛出 400 更符合调用方预期。
    pub const MAX_PER_PAGE: u32 = 200;
    /// 未指定时的每页条数。
    pub const DEFAULT_PER_PAGE: u32 = 20;

    const fn default_page() -> u32 {
        1
    }

    const fn default_per_page() -> u32 {
        Self::DEFAULT_PER_PAGE
    }

    /// 构造分页请求。取值在读取时统一 clamp，此处无需预处理。
    #[must_use]
    pub const fn new(page: u32, per_page: u32) -> Self {
        Self { page, per_page }
    }

    /// 页码，从 1 开始。
    #[must_use]
    pub const fn page(self) -> u32 {
        if self.page == 0 { 1 } else { self.page }
    }

    /// 每页条数，限定在 `1..=MAX_PER_PAGE`。
    #[must_use]
    pub const fn per_page(self) -> u32 {
        if self.per_page == 0 {
            Self::DEFAULT_PER_PAGE
        } else if self.per_page > Self::MAX_PER_PAGE {
            Self::MAX_PER_PAGE
        } else {
            self.per_page
        }
    }

    /// SQL `OFFSET` 值。
    ///
    /// 以 `u64` 计算：`u32` 页码乘以每页条数在极端取值下仍可能溢出 `u32`，
    /// 而 release 构建中的溢出是静默回绕，会直接导致翻到错误的数据页。
    #[must_use]
    pub const fn offset(self) -> u64 {
        (self.page() as u64 - 1) * self.per_page() as u64
    }

    /// SQL `LIMIT` 值。
    #[must_use]
    pub const fn limit(self) -> u64 {
        self.per_page() as u64
    }
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self::new(1, Self::DEFAULT_PER_PAGE)
    }
}

/// 偏移分页结果。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PaginationRes<T> {
    /// 当前页数据。
    pub items: Vec<T>,
    /// 当前页码。
    pub page: u32,
    /// 每页条数。
    pub per_page: u32,
    /// 符合条件的总条数。仅在调用方显式要求时计算。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl<T> PaginationRes<T> {
    /// 构造不含总数的分页结果。
    #[must_use]
    pub const fn new(items: Vec<T>, query: PaginationQuery) -> Self {
        Self {
            items,
            page: query.page(),
            per_page: query.per_page(),
            total: None,
        }
    }

    /// 附加总数。
    #[must_use]
    pub fn with_total(mut self, total: u64) -> Self {
        self.total = Some(total);
        self
    }

    /// 空结果。
    #[must_use]
    pub const fn empty(query: PaginationQuery) -> Self {
        Self {
            items: Vec::new(),
            page: query.page(),
            per_page: query.per_page(),
            total: None,
        }
    }

    /// 逐元素转换，保留分页元信息。
    ///
    /// 用于在接口层把领域实体映射为响应结构。
    pub fn map<U, F: FnMut(T) -> U>(self, f: F) -> PaginationRes<U> {
        PaginationRes {
            items: self.items.into_iter().map(f).collect(),
            page: self.page,
            per_page: self.per_page,
            total: self.total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- PageQuery::page clamp ----

    #[test]
    fn page_zero_falls_back_to_one() {
        let query = PaginationQuery::new(0, 20);
        assert_eq!(query.page(), 1);
    }

    #[test]
    fn page_positive_is_unchanged() {
        let query = PaginationQuery::new(5, 20);
        assert_eq!(query.page(), 5);
    }

    // ---- PageQuery::per_page clamp ----

    #[test]
    fn per_page_zero_falls_back_to_default() {
        let query = PaginationQuery::new(1, 0);
        assert_eq!(query.per_page(), PaginationQuery::DEFAULT_PER_PAGE);
    }

    #[test]
    fn per_page_within_range_is_unchanged() {
        let query = PaginationQuery::new(1, 50);
        assert_eq!(query.per_page(), 50);
    }

    #[test]
    fn per_page_exceeding_max_is_clamped() {
        let query = PaginationQuery::new(1, 999_999);
        assert_eq!(query.per_page(), PaginationQuery::MAX_PER_PAGE);
    }

    #[test]
    fn per_page_exactly_at_max_is_unchanged() {
        let query = PaginationQuery::new(1, PaginationQuery::MAX_PER_PAGE);
        assert_eq!(query.per_page(), PaginationQuery::MAX_PER_PAGE);
    }

    // ---- PageQuery::offset / limit ----

    #[test]
    fn offset_for_first_page_is_zero() {
        let query = PaginationQuery::new(1, 20);
        assert_eq!(query.offset(), 0);
    }

    #[test]
    fn offset_computed_correctly_for_later_page() {
        let query = PaginationQuery::new(3, 20);
        // (3 - 1) * 20 = 40
        assert_eq!(query.offset(), 40);
    }

    #[test]
    fn offset_uses_clamped_per_page_not_raw_input() {
        // per_page 超限会被 clamp 到 MAX_PER_PAGE，offset 应基于 clamp 后的值计算，
        // 而不是原始输入 —— 否则等于绕过了 per_page() 的上限保护。
        let query = PaginationQuery::new(3, 999_999);
        assert_eq!(query.offset(), 2 * PaginationQuery::MAX_PER_PAGE as u64);
    }

    #[test]
    fn offset_does_not_overflow_for_large_page_numbers() {
        // 验证 u64 中间计算能吸收极端页码，不会像 u32 乘法那样静默回绕。
        let query = PaginationQuery::new(u32::MAX, PaginationQuery::MAX_PER_PAGE);
        let expected = (u32::MAX as u64 - 1) * PaginationQuery::MAX_PER_PAGE as u64;
        assert_eq!(query.offset(), expected);
    }

    #[test]
    fn limit_matches_clamped_per_page() {
        let query = PaginationQuery::new(1, 30);
        assert_eq!(query.limit(), 30);
    }

    #[test]
    fn limit_reflects_clamp_when_per_page_zero() {
        let query = PaginationQuery::new(1, 0);
        assert_eq!(query.limit(), PaginationQuery::DEFAULT_PER_PAGE as u64);
    }

    // ---- PageQuery::default ----

    #[test]
    fn default_is_page_one_with_default_per_page() {
        let query = PaginationQuery::default();
        assert_eq!(query.page(), 1);
        assert_eq!(query.per_page(), PaginationQuery::DEFAULT_PER_PAGE);
        assert_eq!(query.offset(), 0);
    }

    // ---- PageQuery deserialize ----

    #[test]
    fn deserializes_from_empty_json_object_using_defaults() {
        let query: PaginationQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.page(), 1);
        assert_eq!(query.per_page(), PaginationQuery::DEFAULT_PER_PAGE);
    }

    #[test]
    fn deserializes_with_page_and_per_page_present() {
        let json = r#"{"page":4,"per_page":50}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page(), 4);
        assert_eq!(query.per_page(), 50);
    }

    #[test]
    fn deserialize_uses_per_page_field_name_not_size() {
        // rename = "per_page" 的行为验证：查询串/JSON 里必须叫 per_page，
        // 而不是内部曾经用过的 size —— 防止字段名回归。
        let json = r#"{"size":50}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        // "size" 不是已知字段，被忽略，per_page 落回默认值
        assert_eq!(query.per_page(), PaginationQuery::DEFAULT_PER_PAGE);
    }

    #[test]
    fn deserialize_still_clamps_oversized_per_page() {
        // 反序列化本身不报错（u32 足够宽，不会像 u16 那样直接拒绝合理的深翻页输入），
        // clamp 发生在取值时。
        let json = r#"{"page":100000,"per_page":100000}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.page(), 100_000);
        assert_eq!(query.per_page(), PaginationQuery::MAX_PER_PAGE);
    }

    // ---- Page<T>::new ----

    #[test]
    fn page_new_stores_items_and_clamped_metadata() {
        let query = PaginationQuery::new(2, 999_999);
        let page = PaginationRes::new(vec![1, 2, 3], query);
        assert_eq!(page.items, vec![1, 2, 3]);
        assert_eq!(page.page, 2);
        assert_eq!(page.per_page, PaginationQuery::MAX_PER_PAGE);
        assert!(page.total.is_none());
    }

    #[test]
    fn page_empty_has_no_items_and_no_total() {
        let query = PaginationQuery::new(1, 20);
        let page: PaginationRes<i32> = PaginationRes::empty(query);
        assert!(page.items.is_empty());
        assert_eq!(page.page, 1);
        assert_eq!(page.per_page, 20);
        assert!(page.total.is_none());
    }

    #[test]
    fn page_with_total_sets_total() {
        let query = PaginationQuery::new(1, 20);
        let page = PaginationRes::new(vec![1, 2], query).with_total(42);
        assert_eq!(page.total, Some(42));
    }

    #[test]
    fn page_map_transforms_items_and_preserves_metadata() {
        let query = PaginationQuery::new(2, 10);
        let page = PaginationRes::new(vec![1, 2, 3], query).with_total(3);
        let mapped = page.map(|n| n.to_string());
        assert_eq!(
            mapped.items,
            vec!["1".to_string(), "2".to_string(), "3".to_string()]
        );
        assert_eq!(mapped.page, 2);
        assert_eq!(mapped.per_page, 10);
        assert_eq!(mapped.total, Some(3));
    }

    // ---- Page<T> serialize ----

    #[test]
    fn page_total_omitted_from_json_when_none() {
        // #[serde(skip_serializing_if = "Option::is_none")] 的行为验证：
        // 未显式计算 total 时响应体里不应出现多余的 null 字段。
        let query = PaginationQuery::new(1, 20);
        let page = PaginationRes::new(vec![1, 2], query);
        let json = serde_json::to_string(&page).unwrap();
        assert!(!json.contains("total"));
    }

    #[test]
    fn page_total_included_when_present() {
        let query = PaginationQuery::new(1, 20);
        let page = PaginationRes::new(vec![1, 2], query).with_total(2);
        let json = serde_json::to_string(&page).unwrap();
        assert!(json.contains("\"total\":2"));
    }

    #[test]
    fn page_serializes_with_per_page_field_name() {
        // 确认响应体字段名同步跟着 per_page 改了，不是只有请求端改了
        // 响应端还留着旧的 size。
        let query = PaginationQuery::new(1, 20);
        let page = PaginationRes::new(vec![1], query);
        let json = serde_json::to_string(&page).unwrap();
        assert!(json.contains("\"per_page\":20"));
        assert!(!json.contains("\"size\""));
    }
}
