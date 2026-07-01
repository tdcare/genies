-- V2: 创建角色表
CREATE TABLE IF NOT EXISTS auth_admin_roles (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE COMMENT '角色标识',
    display_name VARCHAR(128) NOT NULL DEFAULT '' COMMENT '显示名称',
    description VARCHAR(512) DEFAULT '' COMMENT '描述',
    parent_id BIGINT NOT NULL DEFAULT 0 COMMENT '父角色ID (层级继承)',
    status TINYINT NOT NULL DEFAULT 1 COMMENT '状态: 1启用 0禁用',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
    INDEX idx_parent_id (parent_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='角色表';

-- 初始化默认角色
INSERT INTO auth_admin_roles (name, display_name, description, parent_id) VALUES
('admin', '管理员', '系统超级管理员，拥有所有权限', 0),
('operator', '操作员', '日常操作人员', 0),
('viewer', '观察者', '只读权限', 0);
