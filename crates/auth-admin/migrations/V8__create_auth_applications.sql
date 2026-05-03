CREATE TABLE IF NOT EXISTS auth_applications (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    app_name VARCHAR(128) NOT NULL UNIQUE COMMENT '应用标识',
    display_name VARCHAR(256) COMMENT '显示名称',
    description VARCHAR(512) COMMENT '应用描述',
    base_url VARCHAR(256) NOT NULL COMMENT '微服务访问地址',
    status TINYINT(1) DEFAULT 1 COMMENT '1=启用 0=禁用',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
);

-- 默认应用：认证管理中心（自身）
INSERT INTO auth_applications (app_name, display_name, description, base_url, status)
VALUES ('auth-admin', '认证管理中心', '统一认证与权限管理微服务', 'http://localhost:9099', 1)
ON DUPLICATE KEY UPDATE display_name = VALUES(display_name);
