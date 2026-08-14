use platform_kernel::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrganizationMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionMarker;

pub type UserId = Id<UserMarker>;
pub type RoleId = Id<RoleMarker>;
pub type OrganizationId = Id<OrganizationMarker>;
pub type PositionId = Id<PositionMarker>;
pub type PermissionId = Id<PermissionMarker>;
