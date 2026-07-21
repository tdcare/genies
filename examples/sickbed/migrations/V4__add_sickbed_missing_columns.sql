-- 添加 SickbedEntity 缺失的列（与 Java 端 SickbedModel 对齐）
--! may_fail: true
ALTER TABLE SickbedEntity ADD COLUMN sickbedNoAlias VARCHAR(32);
--! may_fail: true
ALTER TABLE SickbedEntity ADD COLUMN remark VARCHAR(255);
--! may_fail: true
ALTER TABLE SickbedEntity ADD COLUMN sickbedType VARCHAR(32);
--! may_fail: true
ALTER TABLE SickbedEntity ADD COLUMN groupNo VARCHAR(32);
