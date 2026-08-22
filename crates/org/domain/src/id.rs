use platform_kernel::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrganizationMarker;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PositionMarker;

pub type OrganizationId = Id<OrganizationMarker>;
pub type PositionId = Id<PositionMarker>;
