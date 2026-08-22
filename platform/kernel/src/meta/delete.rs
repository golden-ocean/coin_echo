use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeleteMeta {
    deleted_at: Option<DateTime<Utc>>,
    deleted_by: Option<Uuid>,
}

impl DeleteMeta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn delete(&self, deleter_id: Option<Uuid>, now: DateTime<Utc>) -> Self {
        if self.deleted_at.is_none() {
            Self {
                deleted_at: Some(now),
                deleted_by: deleter_id,
            }
        } else {
            *self
        }
    }

    pub fn restore(deleted_at: Option<DateTime<Utc>>, deleted_by: Option<Uuid>) -> Self {
        Self {
            deleted_at,
            deleted_by,
        }
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    pub fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    pub fn deleted_by(&self) -> Option<Uuid> {
        self.deleted_by
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// 测试正常删除、重复删除不覆盖历史数据
    #[test]
    fn test_delete_skip_duplicate() {
        let del_meta = DeleteMeta::new();
        let del_user = Some(Uuid::now_v7());

        // 第一次删除：接收返回的新对象
        let now = Utc::now();
        let del_meta = del_meta.delete(del_user, now);
        assert!(del_meta.is_deleted());
        assert_eq!(del_meta.deleted_by(), del_user);
        let first_del_time = del_meta.deleted_at();

        // 第二次删除，更换操作人，时间不变
        let another_user = Some(Uuid::now_v7());
        let now = Utc::now();
        let del_meta = del_meta.delete(another_user, now);

        // 删除人、时间仍为第一次记录，不会被覆盖
        assert_eq!(del_meta.deleted_by(), del_user);
        assert_eq!(del_meta.deleted_at(), first_del_time);
    }

    /// 测试匿名删除（操作人传 None）
    #[test]
    fn test_anonymous_delete() {
        let del_meta = DeleteMeta::new();
        let now = Utc::now();
        let del_meta = del_meta.delete(None, now);

        assert!(del_meta.is_deleted());
        assert!(del_meta.deleted_by().is_none());
    }

    /// 自定义固定时间删除，适配单元测试 Mock 时间场景
    #[test]
    fn test_custom_fixed_time_delete() {
        let del_meta = DeleteMeta::new();
        let fixed_t = DateTime::parse_from_rfc3339("2026-05-01T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let uid = Some(Uuid::now_v7());

        let del_meta = del_meta.delete(uid, fixed_t);
        assert_eq!(del_meta.deleted_at(), Some(fixed_t));
    }

    /// 全新实例默认未删除
    #[test]
    fn test_new_default_undeleted() {
        let del_meta = DeleteMeta::new();
        assert!(!del_meta.is_deleted());
        assert!(del_meta.deleted_at().is_none());
        assert!(del_meta.deleted_by().is_none());
    }

    /// 测试恢复/从持久化层恢复数据
    #[test]
    fn test_restore() {
        let fixed_t = Some(Utc::now());
        let uid = Some(Uuid::now_v7());

        let del_meta = DeleteMeta::restore(fixed_t, uid);
        assert!(del_meta.is_deleted());
        assert_eq!(del_meta.deleted_at(), fixed_t);
        assert_eq!(del_meta.deleted_by(), uid);
    }
}
