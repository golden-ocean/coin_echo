-- =========================================================================
-- 系统字典表 (sys_dictionary)
-- =========================================================================
CREATE TABLE sys_dictionary (
    id UUID PRIMARY KEY,

    name VARCHAR(64) NOT NULL,
    code VARCHAR(64) NOT NULL,
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,

    sort INT NOT NULL DEFAULT 1000,
    remark VARCHAR(500),
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID
);

COMMENT ON TABLE sys_dictionary IS '系统字典表（如：性别、在职状态等枚举类型的元数据）';
COMMENT ON COLUMN sys_dictionary.name IS '字典名称';
COMMENT ON COLUMN sys_dictionary.code IS '字典编码';
COMMENT ON COLUMN sys_dictionary.is_builtin IS '是否系统内置';
COMMENT ON COLUMN sys_dictionary.sort IS '排序';
COMMENT ON COLUMN sys_dictionary.remark IS '备注';
COMMENT ON COLUMN sys_dictionary.status IS '状态';
COMMENT ON COLUMN sys_dictionary.created_at IS '创建时间';
COMMENT ON COLUMN sys_dictionary.updated_at IS '更新时间';
COMMENT ON COLUMN sys_dictionary.created_by IS '创建者';
COMMENT ON COLUMN sys_dictionary.updated_by IS '更新者';

CREATE UNIQUE INDEX uk_sys_dictionary_name ON sys_dictionary (name);
CREATE UNIQUE INDEX uk_sys_dictionary_code ON sys_dictionary (code);
CREATE INDEX idx_sys_dictionary_status ON sys_dictionary (status);

-- =========================================================================
-- 系统字典项表 (sys_dictionary_item)
-- =========================================================================
CREATE TABLE sys_dictionary_item (
    id UUID PRIMARY KEY,

    dictionary_id UUID NOT NULL,
    label VARCHAR(64) NOT NULL,
    value VARCHAR(128) NOT NULL,
    color VARCHAR(32),
    is_builtin BOOLEAN NOT NULL DEFAULT FALSE,
    sort INT NOT NULL DEFAULT 1000,
    remark VARCHAR(500),
    status VARCHAR(32) NOT NULL DEFAULT 'enabled',

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by UUID,
    updated_by UUID
);

COMMENT ON TABLE sys_dictionary_item IS '系统字典项表（字典的具体枚举值）';
COMMENT ON COLUMN sys_dictionary_item.dictionary_id IS '所属字典ID';
COMMENT ON COLUMN sys_dictionary_item.label IS '显示名称';
COMMENT ON COLUMN sys_dictionary_item.value IS '枚举值';
COMMENT ON COLUMN sys_dictionary_item.color IS '前端展示颜色';
COMMENT ON COLUMN sys_dictionary_item.is_builtin IS '是否系统内置';
COMMENT ON COLUMN sys_dictionary_item.sort IS '排序';
COMMENT ON COLUMN sys_dictionary_item.remark IS '备注';
COMMENT ON COLUMN sys_dictionary_item.status IS '状态';
COMMENT ON COLUMN sys_dictionary_item.created_at IS '创建时间';
COMMENT ON COLUMN sys_dictionary_item.updated_at IS '更新时间';
COMMENT ON COLUMN sys_dictionary_item.created_by IS '创建者';
COMMENT ON COLUMN sys_dictionary_item.updated_by IS '更新者';

CREATE UNIQUE INDEX uk_sys_dictionary_item_dict_label
    ON sys_dictionary_item (dictionary_id, label);
CREATE UNIQUE INDEX uk_sys_dictionary_item_dict_value
    ON sys_dictionary_item (dictionary_id, value);
CREATE INDEX idx_sys_dict_item_dictionary_id ON sys_dictionary_item (dictionary_id);
CREATE INDEX idx_sys_dict_item_status ON sys_dictionary_item (status);
