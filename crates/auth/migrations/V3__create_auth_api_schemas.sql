-- API Schema 元数据表
-- 存储 API 端点与字段的结构化元信息，用于动态权限控制
CREATE TABLE IF NOT EXISTS auth_api_schemas (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    schema_name VARCHAR(256) NOT NULL,
    schema_label VARCHAR(256),
    field_name VARCHAR(128) NOT NULL,
    field_label VARCHAR(256),
    field_type VARCHAR(64),
    endpoint_path VARCHAR(256),
    endpoint_label VARCHAR(256),
    http_method VARCHAR(16),
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    UNIQUE KEY uk_schema_field (schema_name, field_name)
);
