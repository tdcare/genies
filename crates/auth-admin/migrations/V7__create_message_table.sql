-- 领域事件 Outbox 模式消息表
CREATE TABLE IF NOT EXISTS message (
    id VARCHAR(36) PRIMARY KEY,
    destination VARCHAR(255),
    headers TEXT,
    payload TEXT NOT NULL,
    published INT DEFAULT 0,
    creation_time BIGINT
);
--! may_fail: true
CREATE INDEX idx_message_published ON message (published);
