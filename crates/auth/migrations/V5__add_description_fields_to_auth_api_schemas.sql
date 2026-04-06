-- 为 auth_api_schemas 表添加 OpenAPI 说明信息字段
--! may_fail: true
ALTER TABLE auth_api_schemas ADD COLUMN schema_description VARCHAR(1024);
--! may_fail: true
ALTER TABLE auth_api_schemas ADD COLUMN field_description VARCHAR(512);
--! may_fail: true
ALTER TABLE auth_api_schemas ADD COLUMN field_required TINYINT(1) DEFAULT 0;
--! may_fail: true
ALTER TABLE auth_api_schemas ADD COLUMN endpoint_description TEXT;
--! may_fail: true
ALTER TABLE auth_api_schemas ADD COLUMN endpoint_tags VARCHAR(512);
--! may_fail: true
ALTER TABLE auth_api_schemas ADD COLUMN endpoint_operation_id VARCHAR(256);
