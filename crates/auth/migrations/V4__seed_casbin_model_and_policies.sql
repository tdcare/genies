-- 初始化 Casbin 模型和策略数据

-- 插入默认 Casbin 模型定义
-- 支持 RBAC + 对象分组 + deny 黑名单模式
INSERT IGNORE INTO casbin_model (model_name, model_text, description) VALUES (
    'default',
    '[request_definition]\nr = sub, obj, act\n\n[policy_definition]\np = sub, obj, act, eft\n\n[role_definition]\ng = _, _\ng2 = _, _\n\n[policy_effect]\ne = !some(where (p.eft == deny))\n\n[matchers]\nm = (g(r.sub, p.sub) || r.sub == p.sub) && (g2(r.obj, p.obj) || r.obj == p.obj || keyMatch2(r.obj, p.obj)) && r.act == p.act',
    'RBAC + 对象分组 + deny 黑名单模型'
);

-- =====================================================
-- 字段级策略规则
-- 控制用户对特定数据字段的访问权限
-- =====================================================
INSERT IGNORE INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES
    ('p', 'alice', 'genies_auth.vo.UserProfile.email', 'read', 'deny'),
    ('p', 'bob', 'genies_auth.vo.User.email', 'read', 'allow'),
    ('p', 'bob', 'genies_auth.vo.UserProfile.email', 'read', 'deny'),
    ('p', 'bob', 'data2', 'write', 'deny'),
    ('p', 'data_group_admin', 'data_group', 'read', 'deny');

-- =====================================================
-- API 接口级策略规则
-- 控制用户对特定 API 端点的访问权限
-- =====================================================
INSERT IGNORE INTO casbin_rules (ptype, v0, v1, v2, v3) VALUES
    ('p', 'guest', '/api/admin', 'get', 'deny'),
    ('p', 'nurse', 'user:manage', 'delete', 'deny');

-- =====================================================
-- 角色分配 (g 类型规则)
-- 将用户分配到角色
-- =====================================================
INSERT IGNORE INTO casbin_rules (ptype, v0, v1) VALUES
    ('g', 'alice', 'data_group_admin');

-- =====================================================
-- 对象分组：字段分组 (g2 类型规则)
-- 将敏感字段归类到数据组
-- =====================================================
INSERT IGNORE INTO casbin_rules (ptype, v0, v1) VALUES
    ('g2', 'genies_auth.vo.UserProfile.credit_card', 'data_group'),
    ('g2', 'genies_auth.vo.UserProfile.name', 'data_group'),
    ('g2', 'genies_auth.vo.User.phone', 'data_group');

-- =====================================================
-- 对象分组：API 路径到操作标识映射 (g2 类型规则)
-- 将 API 路径映射到统一的操作标识符
-- =====================================================
INSERT IGNORE INTO casbin_rules (ptype, v0, v1) VALUES
    ('g2', '/api/users', 'user:manage'),
    ('g2', '/api/users/*', 'user:manage'),
    ('g2', '/api/admin', 'admin:manage'),
    ('g2', '/api/admin/*', 'admin:manage');
