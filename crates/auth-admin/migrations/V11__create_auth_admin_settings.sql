CREATE TABLE IF NOT EXISTS auth_admin_settings (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    setting_key VARCHAR(128) NOT NULL UNIQUE COMMENT '设置键名',
    setting_value JSON NOT NULL COMMENT '设置值(JSON格式)',
    description VARCHAR(256) DEFAULT '' COMMENT '描述',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间'
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='系统设置表';

-- 默认值：所有新功能默认关闭，保持向后兼容
INSERT INTO auth_admin_settings (setting_key, setting_value, description) VALUES
('auth.2fa', '{"enabled":false,"methods":[]}', '双因素认证配置'),
('auth.captcha', '{"enabled":false}', '登录验证码配置'),
('auth.password', '{"min_length":6,"require_uppercase":false,"require_lowercase":false,"require_digit":false,"require_special":false}', '密码强度策略');
