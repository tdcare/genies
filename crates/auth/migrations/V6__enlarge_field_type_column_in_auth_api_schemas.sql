-- 扩大 auth_api_schemas 表中 field_type 列的 varchar 长度
-- 原始长度 VARCHAR(64) 不足以存储复杂嵌套类型描述（如 ApplicationListResult.list），导致 MySQL error 1406 Data too long
ALTER TABLE auth_api_schemas MODIFY COLUMN field_type VARCHAR(2048);
