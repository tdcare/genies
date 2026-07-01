-- V1: 创建用户表
CREATE TABLE IF NOT EXISTS auth_admin_users (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    username VARCHAR(64) NOT NULL UNIQUE COMMENT '用户名',
    password_hash VARCHAR(256) NOT NULL COMMENT 'bcrypt 密码哈希',
    display_name VARCHAR(128) NOT NULL DEFAULT '' COMMENT '显示名称',
    email VARCHAR(128) DEFAULT '' COMMENT '邮箱',
    phone VARCHAR(32) DEFAULT '' COMMENT '手机号',
    avatar VARCHAR(256) DEFAULT '' COMMENT '头像URL',
    status TINYINT NOT NULL DEFAULT 1 COMMENT '状态: 1启用 0禁用',
    last_login_at DATETIME NULL COMMENT '最后登录时间',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='认证用户表';

-- 初始化 admin 用户 (密码: admin123)
INSERT INTO auth_admin_users (username, password_hash, display_name, status) VALUES
('admin', '$2b$12$iMWYLrMbxRjg55k5Dtzkie4Y3hLhnnV6LSQ3vIpHhOcePdsZxiyfW', '超级管理员', 1);
