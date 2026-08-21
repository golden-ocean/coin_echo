-- =========================================================================
-- 系统组织架构表 (org_organization)
-- =========================================================================
CREATE TABLE org_organization (
    id UUID PRIMARY KEY,
    parent_id UUID,

    name VARCHAR(64) NOT NULL,
    code VARCHAR(64) NOT NULL,
    contact VARCHAR(64) NOT NULL DEFAULT '',
    phone VARCHAR(32) NOT NULL DEFAULT '',
    email VARCHAR(128) NOT NULL DEFAULT '',

    sort INT NOT NULL DEFAULT 1000,
    remark VARCHAR(500),
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID
);

COMMENT ON TABLE org_organization IS '系统组织架构表';
COMMENT ON COLUMN org_organization.id IS '组织ID';
COMMENT ON COLUMN org_organization.parent_id IS '父级组织ID，NULL表示顶级';
COMMENT ON COLUMN org_organization.name IS '组织名称';
COMMENT ON COLUMN org_organization.code IS '组织编码';
COMMENT ON COLUMN org_organization.contact IS '联系人';
COMMENT ON COLUMN org_organization.phone IS '联系电话';
COMMENT ON COLUMN org_organization.email IS '联系邮箱';
COMMENT ON COLUMN org_organization.sort IS '排序';
COMMENT ON COLUMN org_organization.remark IS '备注';
COMMENT ON COLUMN org_organization.status IS '状态';
COMMENT ON COLUMN org_organization.created_at IS '创建时间';
COMMENT ON COLUMN org_organization.updated_at IS '更新时间';
COMMENT ON COLUMN org_organization.created_by IS '创建者';
COMMENT ON COLUMN org_organization.updated_by IS '更新者';
COMMENT ON COLUMN org_organization.deleted_at IS '删除时间';
COMMENT ON COLUMN org_organization.deleted_by IS '删除者';

CREATE UNIQUE INDEX uk_org_organization_name ON org_organization (name) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_org_organization_code ON org_organization (code) WHERE deleted_at IS NULL;
CREATE INDEX idx_org_organization_parent_id ON org_organization (parent_id);
CREATE INDEX idx_org_organization_status ON org_organization (status);


-- =========================================================================
-- 系统职位表 (org_position)
-- =========================================================================
CREATE TABLE org_position (
    id UUID PRIMARY KEY,

    name VARCHAR(64) NOT NULL,
    code VARCHAR(64) NOT NULL,

    sort INT NOT NULL DEFAULT 1000,
    remark VARCHAR(500),
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID
);

COMMENT ON TABLE org_position IS '系统职位表';
COMMENT ON COLUMN org_position.id IS '职位ID';
COMMENT ON COLUMN org_position.name IS '职位名称';
COMMENT ON COLUMN org_position.code IS '职位编码';
COMMENT ON COLUMN org_position.sort IS '排序';
COMMENT ON COLUMN org_position.remark IS '备注';
COMMENT ON COLUMN org_position.status IS '状态';
COMMENT ON COLUMN org_position.created_at IS '创建时间';
COMMENT ON COLUMN org_position.updated_at IS '更新时间';
COMMENT ON COLUMN org_position.created_by IS '创建者';
COMMENT ON COLUMN org_position.updated_by IS '更新者';
COMMENT ON COLUMN org_position.deleted_at IS '删除时间';
COMMENT ON COLUMN org_position.deleted_by IS '删除者';


CREATE UNIQUE INDEX uk_org_position_name ON org_position (name) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_org_position_code ON org_position (code) WHERE deleted_at IS NULL;
CREATE INDEX idx_org_position_status ON org_position (status);
