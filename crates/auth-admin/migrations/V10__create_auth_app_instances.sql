CREATE TABLE IF NOT EXISTS auth_app_instances (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    app_name VARCHAR(128) NOT NULL,
    instance_id BIGINT NOT NULL,
    base_url VARCHAR(256) NOT NULL,
    version VARCHAR(64) DEFAULT '',
    status TINYINT NOT NULL DEFAULT 1,
    last_heartbeat_at DATETIME NOT NULL,
    registered_at DATETIME NOT NULL,
    metadata TEXT,
    UNIQUE KEY uk_instance_id (instance_id),
    INDEX idx_app_name (app_name),
    INDEX idx_status (status)
);
