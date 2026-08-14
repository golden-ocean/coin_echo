use chrono::{DateTime, Utc};
use uuid::Uuid;

/// 实体审计元数据
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditMeta {
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    created_by: Option<Uuid>,
    updated_by: Option<Uuid>,
}

impl AuditMeta {
    /// 初始化审计元数据
    /// 创建时创建人=更新人、创建时间=更新时间
    /// # Params
    /// - creator_id: 创建操作人UUID，无则传None
    /// - now: 当前UTC时间戳，统一由上层clock/业务传入便于单元测试Mock
    pub fn new(creator_id: Option<Uuid>, now: DateTime<Utc>) -> Self {
        let creator_uuid = creator_id.map(Into::into);
        Self {
            created_at: now,
            updated_at: now,
            created_by: creator_uuid,
            updated_by: creator_uuid,
        }
    }

    /// 更新审计信息，仅刷新updated_at、updated_by
    /// # Params
    /// - operator_id: 本次更新操作人UUID
    /// - now: 当前UTC时间戳
    pub fn update<T: Into<Uuid>>(&mut self, operator_id: Option<T>, now: DateTime<Utc>) {
        self.updated_at = now;
        self.updated_by = operator_id.map(Into::into);
    }

    /// 从数据库中恢复审计元数据
    pub fn restore(
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        created_by: Option<Uuid>,
        updated_by: Option<Uuid>,
    ) -> Self {
        Self {
            created_at,
            updated_at,
            created_by,
            updated_by,
        }
    }

    /// 获取创建时间
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// 获取最后更新时间
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// 获取创建人ID
    pub fn created_by(&self) -> Option<Uuid> {
        self.created_by
    }

    /// 获取最后更新人ID
    pub fn updated_by(&self) -> Option<Uuid> {
        self.updated_by
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    /// 测试正常带操作人创建、单次更新流程
    #[test]
    fn test_audit_normal_create_and_update() {
        // 生成测试用户UUID
        let creator = Some(Uuid::now_v7());
        let now = Utc::now();
        // 构造审计元数据（自动取当前时间）
        let mut audit = AuditMeta::new(creator, now);

        // 校验创建阶段：创建/更新时间、操作人完全一致
        assert_eq!(audit.created_by(), creator);
        assert_eq!(audit.updated_by(), creator);
        assert_eq!(audit.created_at(), audit.updated_at());

        // 模拟另一个用户执行更新操作
        let updater = Some(Uuid::now_v7());
        let now = Utc::now();
        audit.update(updater, now);

        // 更新后校验：创建信息不变，更新信息变更
        assert_eq!(audit.created_by(), creator);
        assert_eq!(audit.updated_by(), updater);
        // 更新时间 >= 创建时间
        assert!(audit.updated_at() >= audit.created_at());
    }

    /// 测试匿名创建（操作人传None）
    #[test]
    fn test_audit_anonymous_creator() {
        // 匿名创建，无操作人ID
        let now = Utc::now();
        let mut audit = AuditMeta::new(None::<Uuid>, now);

        assert!(audit.created_by().is_none());
        assert!(audit.updated_by().is_none());

        // 匿名更新（显式传递 None）
        let now = Utc::now();
        // 如果使用 Option<T> 泛型版本：
        audit.update(None::<Uuid>, now);

        assert!(audit.updated_by().is_none());
    }

    /// 手动传入固定时间构造
    #[test]
    fn test_audit_custom_fixed_time() {
        let fixed_time = DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let uid = Some(Uuid::now_v7());

        let mut audit = AuditMeta::new(uid, fixed_time);
        assert_eq!(audit.created_at(), fixed_time);
        assert_eq!(audit.updated_at(), fixed_time);

        let new_fixed = DateTime::parse_from_rfc3339("2026-01-02T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        // 匿名更新传 None
        audit.update(None::<Uuid>, new_fixed);
        assert_eq!(audit.updated_at(), new_fixed);
        assert!(audit.updated_by().is_none());
    }
}
