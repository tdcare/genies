-- V5: 创建用户-角色关联表
CREATE TABLE IF NOT EXISTS auth_admin_user_roles (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL COMMENT '用户ID',
    role_id BIGINT NOT NULL COMMENT '角色ID',
    UNIQUE KEY uk_user_role (user_id, role_id),
    INDEX idx_user_id (user_id),
    INDEX idx_role_id (role_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='用户-角色关联表';

-- 初始化: admin 用户分配 admin 角色
INSERT INTO auth_admin_user_roles (user_id, role_id) VALUES (1, 1);
