//! UserTwoFactor 仓储

use rbatis::executor::Executor;
use rbatis::py_sql;

use crate::domain::entity::user_2fa_entity::UserTwoFactor;

/// find by user_id
#[py_sql("select * from auth_admin_user_2fa where user_id = #{user_id}")]
pub async fn find_by_user_id(
    rb: &dyn Executor,
    user_id: &i64,
) -> rbatis::Result<Option<UserTwoFactor>> { impled!() }

/// delete by user_id
#[py_sql("delete from auth_admin_user_2fa where user_id = #{user_id}")]
pub async fn delete_by_user_id(
    rb: &dyn Executor,
    user_id: &i64,
) -> rbatis::Result<u64> { impled!() }

/// delete by id
#[py_sql("delete from auth_admin_user_2fa where id = #{id}")]
pub async fn delete_by_id(
    rb: &dyn Executor,
    id: &i64,
) -> rbatis::Result<u64> { impled!() }

/// update enabled + backup_codes by id
#[py_sql("update auth_admin_user_2fa set enabled = #{enabled}, backup_codes = #{backup_codes}, updated_at = CURRENT_TIMESTAMP where id = #{id}")]
pub async fn update_enabled_and_backup_codes(
    rb: &dyn Executor,
    id: &i64,
    enabled: &i8,
    backup_codes: &str,
) -> rbatis::Result<u64> { impled!() }

/// update backup_codes by id
#[py_sql("update auth_admin_user_2fa set backup_codes = #{backup_codes}, updated_at = CURRENT_TIMESTAMP where id = #{id}")]
pub async fn update_backup_codes(
    rb: &dyn Executor,
    id: &i64,
    backup_codes: &str,
) -> rbatis::Result<u64> { impled!() }
