-- Casbin 模型定义表
-- 存储 Casbin 的 model.conf 配置，支持动态切换模型
CREATE TABLE IF NOT EXISTS casbin_model (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    model_name VARCHAR(128) NOT NULL DEFAULT 'default',
    model_text TEXT NOT NULL,
    description VARCHAR(512),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uk_model_name (model_name)
);
