-- V6: 创建角色-权限关联表
CREATE TABLE IF NOT EXISTS auth_admin_role_permissions (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    role_id BIGINT NOT NULL COMMENT '角色ID',
    permission_id BIGINT NOT NULL COMMENT '权限ID',
    UNIQUE KEY uk_role_perm (role_id, permission_id),
    INDEX idx_role_id (role_id),
    INDEX idx_perm_id (permission_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='角色-权限关联表';

-- 初始化: admin 角色分配所有权限
INSERT INTO auth_admin_role_permissions (role_id, permission_id) VALUES
(1, 1);
