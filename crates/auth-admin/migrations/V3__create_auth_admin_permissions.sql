-- V3: 创建权限表
CREATE TABLE IF NOT EXISTS auth_admin_permissions (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(128) NOT NULL COMMENT '权限名称',
    resource VARCHAR(256) NOT NULL COMMENT '资源路径',
    action VARCHAR(16) NOT NULL COMMENT '操作: GET/POST/PUT/DELETE/*',
    description VARCHAR(512) DEFAULT '' COMMENT '描述',
    status TINYINT NOT NULL DEFAULT 1 COMMENT '状态: 1启用 0禁用',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
    UNIQUE KEY uk_resource_action (resource, action)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='权限表';

-- 初始化默认权限
INSERT INTO auth_admin_permissions (name, resource, action, description) VALUES
('所有资源', '*', '*', '超级管理员通配权限'),
('用户管理', '/api/users', '*', '用户管理全部操作'),
('角色管理', '/api/roles', '*', '角色管理全部操作'),
('权限管理', '/api/permissions', '*', '权限管理全部操作'),
('部门管理', '/api/departments', '*', '部门管理全部操作'),
('查看仪表盘', '/api/dashboard', 'GET', '查看仪表盘');
