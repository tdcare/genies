//! OAuth Refresh Token Repository

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::oauth_refresh_token_entity::OAuthRefreshTokenEntity;

impl OAuthRefreshTokenEntity {
    #[py_sql("SELECT * FROM auth_oauth_refresh_tokens WHERE token_hash = #{token_hash}")]
    pub async fn find_by_token_hash(rb: &dyn Executor, token_hash: &str) -> rbatis::Result<Option<OAuthRefreshTokenEntity>> {
        impled!()
    }

    #[py_sql("UPDATE auth_oauth_refresh_tokens SET revoked = 1 WHERE id = #{id}")]
    pub async fn revoke_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("UPDATE auth_oauth_refresh_tokens SET revoked = 1 WHERE access_token_id = #{access_token_id}")]
    pub async fn revoke_chain(rb: &dyn Executor, access_token_id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_oauth_refresh_tokens WHERE expires_at < NOW()")]
    pub async fn delete_expired(rb: &dyn Executor) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
