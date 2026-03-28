-- 为 auth_api_schemas 表添加 OpenAPI 说明信息字段
ALTER TABLE auth_api_schemas
  ADD COLUMN schema_description VARCHAR(1024) COMMENT 'Schema对象描述，来自OpenAPI Schema.description',
  ADD COLUMN field_description VARCHAR(512) COMMENT '字段描述，来自OpenAPI property.description',
  ADD COLUMN field_required TINYINT(1) DEFAULT 0 COMMENT '字段是否必需，来自OpenAPI Schema.required',
  ADD COLUMN endpoint_description TEXT COMMENT 'API操作详细描述，来自OpenAPI Operation.description',
  ADD COLUMN endpoint_tags VARCHAR(512) COMMENT 'API标签，JSON数组格式，来自OpenAPI Operation.tags',
  ADD COLUMN endpoint_operation_id VARCHAR(256) COMMENT '操作ID，来自OpenAPI Operation.operationId';
