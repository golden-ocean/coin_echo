use platform_kernel::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionMarker;

pub type UserId = Id<UserMarker>;
pub type RoleId = Id<RoleMarker>;
pub type PermissionId = Id<PermissionMarker>;

use uuid::Uuid;

/// 引用型 ID —— 表示"这里存的是 org 模块下某个 Organization 的 ID"，
/// 但 iam_domain 不依赖 org_domain，也不关心那边的 Organization 内部结构。
/// 只用于 User 聚合内的字段类型标注和跨模块传参，不提供业务校验方法。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OrganizationId(Uuid);

impl OrganizationId {
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for OrganizationId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<OrganizationId> for Uuid {
    fn from(id: OrganizationId) -> Self {
        id.0
    }
}

/// 同理，引用型 PositionId
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PositionId(Uuid);

impl PositionId {
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl From<Uuid> for PositionId {
    fn from(id: Uuid) -> Self {
        Self(id)
    }
}

impl From<PositionId> for Uuid {
    fn from(id: PositionId) -> Self {
        id.0
    }
}
