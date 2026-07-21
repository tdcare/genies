use rbatis::executor::Executor;
use rbatis::py_sql;
use crate::domain::aggregate::WardEntity;
use crate::model::vo::{WardBriefVo, WardSickbedJoin};

// CRUD 宏已在 WardEntity 定义处调用: insert, select_by_column, update_by_column, delete_by_column

impl WardEntity {
    /// 按科室查询，按orderId升序
    #[py_sql("SELECT * FROM WardEntity WHERE departmentId = #{department_id} ORDER BY orderId ASC")]
    pub async fn find_by_department_id_order_by_order_id(
        rb: &dyn Executor,
        department_id: &str,
    ) -> rbatis::Result<Vec<WardEntity>> {
        impled!()
    }

    /// 科室有效病房列表
    #[py_sql("SELECT * FROM WardEntity WHERE departmentId = #{department_id} AND effectiveness = 1 ORDER BY orderId ASC")]
    pub async fn effectiveness_wards(
        rb: &dyn Executor,
        department_id: &str,
    ) -> rbatis::Result<Vec<WardEntity>> {
        impled!()
    }

    /// 科室有效病房简要信息 (id, wardName)
    #[py_sql("SELECT id, wardName FROM WardEntity WHERE departmentId = #{department_id} AND effectiveness = 1 ORDER BY orderId ASC")]
    pub async fn effectiveness_ward_vo(
        rb: &dyn Executor,
        department_id: &str,
    ) -> rbatis::Result<Vec<WardBriefVo>> {
        impled!()
    }

    /// 病房-床位联表查询（有效病房及其下的床位）
    #[py_sql("
        SELECT w.id AS wardId, w.wardName, w.wardNo,
               s.id AS sickbedId, s.sickbedNo, s.patientId, s.status, s.orderId
        FROM WardEntity w
        LEFT JOIN SickbedEntity s ON w.id = s.wardId AND s.effectiveness = 1
        WHERE w.departmentId = #{department_id} AND w.effectiveness = 1
        ORDER BY w.orderId ASC, s.orderId ASC
    ")]
    pub async fn effectiveness_wards_sickbeds(
        rb: &dyn Executor,
        department_id: &str,
    ) -> rbatis::Result<Vec<WardSickbedJoin>> {
        impled!()
    }

    /// 按病房号查询
    #[py_sql("SELECT * FROM WardEntity WHERE wardNo = #{ward_no} LIMIT 1")]
    pub async fn find_by_ward_no(
        rb: &dyn Executor,
        ward_no: &str,
    ) -> rbatis::Result<Vec<WardEntity>> {
        impled!()
    }

    /// 分页搜索（条件过滤）
    #[py_sql("
        SELECT * FROM WardEntity
        WHERE 1=1
        if department_id != '':
            AND departmentId = #{department_id}
        if ward_name != '':
            AND wardName LIKE concat('%',#{ward_name},'%')
        if ward_type != '':
            AND wardType = #{ward_type}
        ORDER BY orderId ASC
    ")]
    pub async fn effectiveness_for_search(
        rb: &dyn Executor,
        department_id: &str,
        ward_name: &str,
        ward_type: &str,
    ) -> rbatis::Result<Vec<WardEntity>> {
        impled!()
    }
}
