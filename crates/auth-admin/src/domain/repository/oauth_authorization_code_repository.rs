//! OAuth 授权码 Repository

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::oauth_authorization_code_entity::OAuthAuthorizationCodeEntity;

impl OAuthAuthorizationCodeEntity {
    #[py_sql("SELECT * FROM auth_oauth_authorization_codes WHERE code = #{code}")]
    pub async fn find_by_code(rb: &dyn Executor, code: &str) -> rbatis::Result<Option<OAuthAuthorizationCodeEntity>> {
        impled!()
    }

    #[py_sql("UPDATE auth_oauth_authorization_codes SET used = 1 WHERE id = #{id} AND used = 0")]
    pub async fn mark_used(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_oauth_authorization_codes WHERE expires_at < NOW()")]
    pub async fn delete_expired(rb: &dyn Executor) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
