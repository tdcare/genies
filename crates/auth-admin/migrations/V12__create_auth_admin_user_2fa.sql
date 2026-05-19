CREATE TABLE IF NOT EXISTS auth_admin_user_2fa (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    user_id BIGINT NOT NULL UNIQUE COMMENT '用户ID',
    method VARCHAR(32) NOT NULL DEFAULT '' COMMENT '2FA方式: totp / sms / second_password',
    enabled TINYINT NOT NULL DEFAULT 0 COMMENT '是否启用',
    secret VARCHAR(256) DEFAULT '' COMMENT '加密后的TOTP密钥或bcrypt二次密码哈希',
    phone VARCHAR(32) DEFAULT '' COMMENT 'SMS方式使用的手机号',
    backup_codes TEXT COMMENT '备用恢复码(JSON数组, bcrypt哈希)',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_user_2fa_user_id (user_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='用户双因素认证配置表';
