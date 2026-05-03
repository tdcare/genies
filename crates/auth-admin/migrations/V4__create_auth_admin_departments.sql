-- V4: 创建部门表
CREATE TABLE IF NOT EXISTS auth_admin_departments (
    id BIGINT AUTO_INCREMENT PRIMARY KEY,
    name VARCHAR(128) NOT NULL COMMENT '部门名称',
    parent_id BIGINT NOT NULL DEFAULT 0 COMMENT '父部门ID',
    sort_order INT NOT NULL DEFAULT 0 COMMENT '排序',
    description VARCHAR(512) DEFAULT '' COMMENT '描述',
    status TINYINT NOT NULL DEFAULT 1 COMMENT '状态: 1启用 0禁用',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '创建时间',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新时间',
    INDEX idx_parent_id (parent_id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COMMENT='部门/组织架构表';

-- 初始化默认部门
INSERT INTO auth_admin_departments (name, parent_id, sort_order, description) VALUES
('总公司', 0, 1, '公司总部'),
('技术部', 1, 1, '技术研发部门'),
('产品部', 1, 2, '产品设计部门'),
('运营部', 1, 3, '运营管理部门');
