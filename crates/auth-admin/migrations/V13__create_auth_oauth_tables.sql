-- ============================================================================
-- OAuth 2.0 认证服务器 — 客户端、授权码、令牌表
-- ============================================================================

-- ----------------------------------------------------------------------------
-- OAuth 客户端注册表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS auth_oauth_clients (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    client_id VARCHAR(64) NOT NULL UNIQUE COMMENT 'OAuth2客户端标识（公开）',
    client_secret_hash VARCHAR(256) NOT NULL COMMENT 'bcrypt哈希的客户端密钥',
    application_id BIGINT NOT NULL COMMENT '关联 auth_applications.id',
    client_name VARCHAR(128) NOT NULL COMMENT '客户端显示名称',
    redirect_uris TEXT NOT NULL COMMENT 'JSON数组：允许的回调地址',
    grant_types VARCHAR(256) NOT NULL DEFAULT '["authorization_code","refresh_token"]' COMMENT 'JSON数组：允许的授权模式',
    scopes VARCHAR(512) NOT NULL DEFAULT '["openid","profile"]' COMMENT 'JSON数组：允许的权限范围',
    token_format VARCHAR(16) NOT NULL DEFAULT 'jwt' COMMENT 'Token格式：jwt 或 opaque',
    access_token_ttl INT NOT NULL DEFAULT 3600 COMMENT 'Access Token有效期（秒）',
    refresh_token_ttl INT NOT NULL DEFAULT 2592000 COMMENT 'Refresh Token有效期（秒，默认30天）',
    require_pkce TINYINT NOT NULL DEFAULT 0 COMMENT '是否强制PKCE：1=必须 0=可选',
    status TINYINT NOT NULL DEFAULT 1 COMMENT '1=启用 0=禁用',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
    INDEX idx_application_id (application_id),
    INDEX idx_status (status)
) COMMENT='OAuth2客户端注册表';

-- ----------------------------------------------------------------------------
-- OAuth 授权码表（authorization_code grant）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS auth_oauth_authorization_codes (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    code VARCHAR(128) NOT NULL UNIQUE COMMENT '授权码（原始值，一次性使用）',
    client_id BIGINT NOT NULL COMMENT '关联 auth_oauth_clients.id',
    user_id BIGINT COMMENT '授权用户（NULL为client_credentials）',
    redirect_uri VARCHAR(1024) NOT NULL COMMENT '使用的回调地址',
    scopes VARCHAR(512) COMMENT '授予的权限范围',
    code_challenge VARCHAR(128) COMMENT 'PKCE code_challenge',
    code_challenge_method VARCHAR(8) DEFAULT 'S256' COMMENT 'PKCE方法',
    expires_at DATETIME NOT NULL COMMENT '过期时间（10分钟）',
    used TINYINT NOT NULL DEFAULT 0 COMMENT '1=已使用（防重放）',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_code (code),
    INDEX idx_client_id (client_id),
    INDEX idx_expires_at (expires_at)
) COMMENT='OAuth2授权码表';

-- ----------------------------------------------------------------------------
-- OAuth Access Token 表（仅 opaque 模式持久化，JWT为无状态）
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS auth_oauth_access_tokens (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    token_hash VARCHAR(128) NOT NULL UNIQUE COMMENT 'SHA256(access_token)',
    client_id BIGINT NOT NULL COMMENT '关联 auth_oauth_clients.id',
    user_id BIGINT COMMENT '关联 auth_admin_users.id',
    scopes VARCHAR(512) COMMENT '授予的权限范围',
    expires_at DATETIME NOT NULL COMMENT '过期时间',
    revoked TINYINT NOT NULL DEFAULT 0 COMMENT '1=已撤销',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_client_id (client_id),
    INDEX idx_user_id (user_id),
    INDEX idx_expires_at (expires_at)
) COMMENT='OAuth2 Access Token表（opaque模式）';

-- ----------------------------------------------------------------------------
-- OAuth Refresh Token 表
-- ----------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS auth_oauth_refresh_tokens (
    id BIGINT PRIMARY KEY AUTO_INCREMENT,
    token_hash VARCHAR(128) NOT NULL UNIQUE COMMENT 'SHA256(refresh_token)',
    client_id BIGINT NOT NULL COMMENT '关联 auth_oauth_clients.id',
    user_id BIGINT COMMENT '关联 auth_admin_users.id',
    scopes VARCHAR(512) COMMENT '授予的权限范围',
    access_token_id BIGINT COMMENT '关联 auth_oauth_access_tokens.id（JWT模式为NULL）',
    expires_at DATETIME NOT NULL COMMENT '过期时间',
    revoked TINYINT NOT NULL DEFAULT 0 COMMENT '1=已撤销',
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    INDEX idx_client_id (client_id),
    INDEX idx_user_id (user_id),
    INDEX idx_access_token_id (access_token_id),
    INDEX idx_expires_at (expires_at)
) COMMENT='OAuth2 Refresh Token表';

-- ----------------------------------------------------------------------------
-- 种子数据：默认 admin-ui OAuth 客户端
-- ----------------------------------------------------------------------------
INSERT INTO auth_oauth_clients (client_id, client_secret_hash, application_id, client_name, redirect_uris, grant_types, scopes, token_format, access_token_ttl, refresh_token_ttl, require_pkce, status)
VALUES (
    'admin-ui',
    '$2b$12$LJ3m4ys3Gwo3GQ1GVsRIIO8FhSjDWxB1f1LMDvRQq0RNv9yXOZsxe',  -- bcrypt("admin-ui-secret")
    (SELECT id FROM auth_applications WHERE app_name = 'auth-admin' LIMIT 1),
    '管理后台',
    '["http://localhost:9099/ui/"]',
    '["authorization_code","password","refresh_token"]',
    '["openid","profile","read","write","admin"]',
    'jwt',
    7200,
    2592000,
    0,
    1
) ON DUPLICATE KEY UPDATE client_name = VALUES(client_name);
