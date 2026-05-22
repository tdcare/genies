//! OAuth 客户端 Repository

use rbatis::executor::Executor;
use rbatis::py_sql;
use rbatis::rbdc::db::ExecResult;
use crate::domain::entity::oauth_client_entity::OAuthClientEntity;

impl OAuthClientEntity {
    #[py_sql("SELECT * FROM auth_oauth_clients WHERE client_id = #{client_id}")]
    pub async fn find_by_client_id(rb: &dyn Executor, client_id: &str) -> rbatis::Result<Vec<OAuthClientEntity>> {
        impled!()
    }

    #[py_sql("SELECT * FROM auth_oauth_clients WHERE id = #{id}")]
    pub async fn find_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<Vec<OAuthClientEntity>> {
        impled!()
    }

    #[py_sql("
        SELECT * FROM auth_oauth_clients
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND (client_name LIKE concat('%',#{keyword},'%') OR client_id LIKE concat('%',#{keyword},'%'))`
        ` ORDER BY id DESC`
    ")]
    pub async fn list(rb: &dyn Executor, keyword: &str) -> rbatis::Result<Vec<OAuthClientEntity>> {
        impled!()
    }

    #[py_sql("
        SELECT COUNT(*) AS cnt FROM auth_oauth_clients
        WHERE 1=1
        if keyword != null && keyword != '':
            ` AND (client_name LIKE concat('%',#{keyword},'%') OR client_id LIKE concat('%',#{keyword},'%'))`
    ")]
    pub async fn count(rb: &dyn Executor, keyword: &str) -> rbatis::Result<u64> {
        impled!()
    }

    #[py_sql("UPDATE auth_oauth_clients SET client_secret_hash=#{client_secret_hash} WHERE id=#{id}")]
    pub async fn update_secret_hash(
        rb: &dyn Executor,
        id: &i64,
        client_secret_hash: &str,
    ) -> rbatis::Result<ExecResult> {
        impled!()
    }

    #[py_sql("DELETE FROM auth_oauth_clients WHERE id = #{id}")]
    pub async fn delete_by_id(rb: &dyn Executor, id: &i64) -> rbatis::Result<ExecResult> {
        impled!()
    }
}
