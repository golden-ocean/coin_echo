-- =========================================================================
-- 工号 PostgreSQL Sequence 序列 (用于 StaffNoGenerator)
-- =========================================================================
CREATE SEQUENCE IF NOT EXISTS seq_iam_user_staff_no
    START WITH 100001
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 20;

COMMENT ON SEQUENCE seq_iam_user_staff_no IS '员工工号自增序列号';

-- =========================================================================
-- 系统用户主表 (iam_user)
-- =========================================================================
CREATE TABLE iam_user (
    id UUID PRIMARY KEY,

    -- 基础信息
    username VARCHAR(64) NOT NULL,
    staff_no VARCHAR(64) NOT NULL,
    name VARCHAR(64) NOT NULL,
    email VARCHAR(128) NOT NULL,
    phone VARCHAR(32) NOT NULL,
    gender VARCHAR(16) NOT NULL DEFAULT 'unknown',  -- 枚举 (unknown, male, female)
    birthday DATE,
    avatar VARCHAR(500),

    -- 安全凭证
    password_hash VARCHAR(255) NOT NULL,
    password_updated_at TIMESTAMPTZ NOT NULL,

    -- 业务状态与控制字段 (对应枚举)
    employment_status VARCHAR(32) NOT NULL DEFAULT 'active',  -- 枚举 (active, on_leave, resigned, terminated)
    data_scope VARCHAR(32) NOT NULL DEFAULT 'self_only',  -- 枚举 (self_only, department, department_and_children, custom, all)
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    sort INT NOT NULL DEFAULT 1000,
    remark VARCHAR(500),
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',  -- 枚举 (disabled, enabled)

    -- 组织架构关联 (逻辑外键)
    organization_id UUID,
    position_id UUID,

    -- 审计与软删除字段
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID,
    version BIGINT NOT NULL DEFAULT 0
);

COMMENT ON TABLE iam_user IS '系统用户主表';
COMMENT ON COLUMN iam_user.username IS '登录账号';
COMMENT ON COLUMN iam_user.staff_no IS '员工工号';
COMMENT ON COLUMN iam_user.name IS '名字';
COMMENT ON COLUMN iam_user.email IS '电子邮箱';
COMMENT ON COLUMN iam_user.phone IS '电话号码';
COMMENT ON COLUMN iam_user.gender IS '性别';
COMMENT ON COLUMN iam_user.birthday IS '生日';
COMMENT ON COLUMN iam_user.avatar IS '头像';
COMMENT ON COLUMN iam_user.password_hash IS '密码哈希';
COMMENT ON COLUMN iam_user.password_updated_at IS '密码更新时间';
COMMENT ON COLUMN iam_user.employment_status IS '在职状态';
COMMENT ON COLUMN iam_user.data_scope IS '数据权限范围';
COMMENT ON COLUMN iam_user.is_builtin IS '是否系统内置不可删除';
COMMENT ON COLUMN iam_user.sort IS '排序';
COMMENT ON COLUMN iam_user.remark IS '备注';
COMMENT ON COLUMN iam_user.status IS '状态';
COMMENT ON COLUMN iam_user.organization_id IS '组织ID';
COMMENT ON COLUMN iam_user.position_id IS '职位ID';
COMMENT ON COLUMN iam_user.created_at IS '创建时间';
COMMENT ON COLUMN iam_user.updated_at IS '更新时间';
COMMENT ON COLUMN iam_user.created_by IS '创建者';
COMMENT ON COLUMN iam_user.updated_by IS '更新者';
COMMENT ON COLUMN iam_user.deleted_at IS '删除时间';
COMMENT ON COLUMN iam_user.deleted_by IS '删除者';
COMMENT ON COLUMN iam_user.version IS '版本号';

-- 唯一不变量索引 (软删除隔离)
CREATE UNIQUE INDEX uk_iam_user_username ON iam_user (username) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_iam_user_staff_no ON iam_user (staff_no) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_iam_user_email ON iam_user (email) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_iam_user_phone ON iam_user (phone) WHERE deleted_at IS NULL;
-- 树形与普通关系索引
CREATE INDEX idx_iam_user_org_id ON iam_user (organization_id);
CREATE INDEX idx_iam_user_pos_id ON iam_user (position_id);



-- =========================================================================
-- 系统角色主表 (iam_role)
-- =========================================================================
CREATE TABLE iam_role (
    -- 主键 (对应 RoleId)
    id UUID PRIMARY KEY,

    -- 基础信息
    name VARCHAR(64) NOT NULL,
    code VARCHAR(64) NOT NULL,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    remark VARCHAR(500),
    sort INT NOT NULL DEFAULT 1000,
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID,
    version BIGINT NOT NULL DEFAULT 0
);

COMMENT ON TABLE iam_role IS '系统角色表';
COMMENT ON COLUMN iam_role.name IS '角色名称 (如: 超级管理员)';
COMMENT ON COLUMN iam_role.code IS '角色编码 (如: ROOT, ADMIN)';
COMMENT ON COLUMN iam_role.is_builtin IS '是否为系统内置角色 (内置角色不可被删除和随意修改)';
COMMENT ON COLUMN iam_role.sort IS '排序';
COMMENT ON COLUMN iam_role.remark IS '备注';
COMMENT ON COLUMN iam_role.status IS '状态';
COMMENT ON COLUMN iam_role.created_at IS '创建时间';
COMMENT ON COLUMN iam_role.updated_at IS '更新时间';
COMMENT ON COLUMN iam_role.created_by IS '创建者';
COMMENT ON COLUMN iam_role.updated_by IS '更新者';
COMMENT ON COLUMN iam_role.deleted_at IS '删除时间';
COMMENT ON COLUMN iam_role.deleted_by IS '删除者';
COMMENT ON COLUMN iam_role.version IS '版本号';


CREATE UNIQUE INDEX uk_iam_role_name ON iam_role (name) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_iam_role_code ON iam_role (code) WHERE deleted_at IS NULL;



-- =========================================================================
-- 系统 用户-角色 中间表 (iam_user_role)
-- =========================================================================
CREATE TABLE iam_user_role (
    user_id UUID NOT NULL,
    role_id UUID NOT NULL,
    -- 联合主键防重复插入
    PRIMARY KEY (user_id, role_id)
);

COMMENT ON TABLE iam_user_role IS '用户与角色的多对多映射表';

-- 提升以 role_id 反查 user_id 的性能
CREATE INDEX idx_iam_user_role_role_id ON iam_user_role (role_id);


-- =========================================================================
-- 系统 权限 表 (iam_permission)
-- =========================================================================
CREATE TABLE iam_permission (
    -- 主键 (对应 PermissionId)
    id UUID PRIMARY KEY,

    -- 树形结构父级 ID (逻辑外键，指向本表的 id)
    parent_id UUID,

    -- 基础信息
    name VARCHAR(64) NOT NULL,
    code VARCHAR(128) NOT NULL,

    -- 权限类型 (对应 PermissionKind 枚举，如 Menu, Button, Api)
    kind VARCHAR(32) NOT NULL,

    -- ==========================================
    -- 附属信息：前端路由/菜单专属字段
    -- ==========================================
    route_path VARCHAR(255),    -- 路由地址 (如: /system/user)
    component VARCHAR(255),     -- 前端组件路径 (如: views/system/user/index)
    icon VARCHAR(128),          -- 菜单图标

    -- ==========================================
    -- 附属信息：后端接口专属字段
    -- ==========================================
    api_method VARCHAR(16),     -- 请求方法 (如: GET, POST, PUT, DELETE)
    api_path VARCHAR(255),      -- 后端接口路径 (如: /api/v1/users)

    -- 控制与状态
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    remark VARCHAR(500),
    sort INT NOT NULL DEFAULT 1000,
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID,
    deleted_at TIMESTAMPTZ,
    deleted_by UUID,
    version BIGINT NOT NULL DEFAULT 0
);

COMMENT ON TABLE iam_permission IS '系统权限/菜单表';
COMMENT ON COLUMN iam_permission.parent_id IS '父级权限ID，NULL表示顶级';
COMMENT ON COLUMN iam_permission.name IS '权限名称 (如: 用户管理, 新增用户)';
COMMENT ON COLUMN iam_permission.code IS '权限标识/编码 (如: iam:user:add)';
COMMENT ON COLUMN iam_permission.kind IS '权限类型 (menu:菜单, button:按钮, api:接口)';
COMMENT ON COLUMN iam_permission.is_builtin IS '是否为系统内置权限 (防止超管不慎删除基础菜单)';
COMMENT ON COLUMN iam_permission.route_path IS '前端路由路径';
COMMENT ON COLUMN iam_permission.component IS '前端组件路径';
COMMENT ON COLUMN iam_permission.icon IS '菜单图标';
COMMENT ON COLUMN iam_permission.api_method IS '后端接口请求方法';
COMMENT ON COLUMN iam_permission.api_path IS '后端接口路径';
COMMENT ON COLUMN iam_permission.remark IS '备注';
COMMENT ON COLUMN iam_permission.sort IS '排序';
COMMENT ON COLUMN iam_permission.status IS '状态';
COMMENT ON COLUMN iam_permission.created_at IS '创建时间';
COMMENT ON COLUMN iam_permission.updated_at IS '更新时间';
COMMENT ON COLUMN iam_permission.created_by IS '创建者';
COMMENT ON COLUMN iam_permission.updated_by IS '更新者';
COMMENT ON COLUMN iam_permission.deleted_at IS '删除时间';
COMMENT ON COLUMN iam_permission.deleted_by IS '删除者';
COMMENT ON COLUMN iam_permission.version IS '版本号';


CREATE UNIQUE INDEX uk_iam_permission_code ON iam_permission (code) WHERE deleted_at IS NULL;
CREATE UNIQUE INDEX uk_iam_permission_name ON iam_permission (name) WHERE deleted_at IS NULL;
-- 树形结构高频查询优化：根据父节点查子节点
CREATE INDEX idx_iam_permission_parent_id ON iam_permission (parent_id);


-- =========================================================================
-- 系统 角色-权限 中间表 (iam_role_permission)
-- =========================================================================
CREATE TABLE iam_role_permission (
    role_id UUID NOT NULL,
    permission_id UUID NOT NULL,
    -- 联合主键防止重复分配相同权限
    PRIMARY KEY (role_id, permission_id)
);

COMMENT ON TABLE iam_role_permission IS '角色与权限的多对多映射表';

-- 提升以 permission_id 反查 role_id 的性能 (比如查询“谁拥有某某权限”时)
CREATE INDEX idx_iam_role_permission_permission_id ON iam_role_permission (permission_id);
