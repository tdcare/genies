//! Ward 业务域接口对比测试
//!
//! 运行方式：
//! ```bash
//! JAVA_BASE_URL=http://localhost:8081 RUST_BASE_URL=http://localhost:8080 \
//!   cargo test -p sickbed --test ward_comparison_tests -- --nocapture
//! ```

mod common;

use common::*;
use serde_json::Value;

// ==================== Ward GET 端点对比 ====================

#[tokio::test]
async fn compare_ward_get_find_by_id() {
    let client = http_client();
    let id = test_ward_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!("{}/ward/id/{}", java_base_url(), id))
            .send(),
        client
            .get(format!("{}/ward/id/{}", rust_base_url(), id))
            .send(),
    );

    let java_json: Value = java_resp.unwrap().json().await.unwrap();
    let rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    let diffs = deep_diff("ward.findById", &java_json, &rust_json);
    assert_no_diffs("ward.findById", &diffs);
}

#[ignore]
#[tokio::test]
async fn compare_ward_get_find_ward_relevance_sickbed() {
    let client = http_client();
    let id = test_ward_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/ward/findWardRelevnceSickbe?id={}",
                java_base_url(),
                id
            ))
            .send(),
        client
            .get(format!(
                "{}/ward/findWardRelevnceSickbe?id={}",
                rust_base_url(),
                id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("ward.findWardRelevnceSickbe", &java_json, &rust_json);
    assert_no_diffs("ward.findWardRelevnceSickbe", &diffs);
}

#[tokio::test]
async fn compare_ward_get_effectiveness() {
    let client = http_client();
    let dept_id = test_dept_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/ward/effectiveness?departmentId={}",
                java_base_url(),
                dept_id
            ))
            .send(),
        client
            .get(format!(
                "{}/ward/effectiveness?departmentId={}",
                rust_base_url(),
                dept_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("ward.effectiveness", &java_json, &rust_json);
    assert_no_diffs("ward.effectiveness", &diffs);
}

#[tokio::test]
async fn compare_ward_get_effectiveness_for_search() {
    let client = http_client();
    let dept_id = test_dept_id();
    let qs = format!(
        "departmentId={}&wardName=&wardType=&pageIndex=0&pageSize=10",
        dept_id
    );

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/ward/effectivenessForSearch?{}",
                java_base_url(),
                qs
            ))
            .send(),
        client
            .get(format!(
                "{}/ward/effectivenessForSearch?{}",
                rust_base_url(),
                qs
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("ward.effectivenessForSearch", &java_json, &rust_json);
    assert_no_diffs("ward.effectivenessForSearch", &diffs);
}

#[tokio::test]
async fn compare_ward_get_effectiveness_ward_vo() {
    let client = http_client();
    let dept_id = test_dept_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/ward/effectivenessWardVo?departmentId={}",
                java_base_url(),
                dept_id
            ))
            .send(),
        client
            .get(format!(
                "{}/ward/effectivenessWardVo?departmentId={}",
                rust_base_url(),
                dept_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("ward.effectivenessWardVo", &java_json, &rust_json);
    assert_no_diffs("ward.effectivenessWardVo", &diffs);
}

// ==================== Ward POST 端点对比 ====================

#[tokio::test]
async fn compare_ward_post_add() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "ward": {
            "wardName": "CMP-TEST-WARD",
            "wardType": "普通",
            "departmentId": test_dept_id(),
            "effectiveness": 1
        },
        "sickbeds": []
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "ward.add",
        &format!("{}/ward/add", java_base_url()),
        &format!("{}/ward/add", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "WardEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("wardName = 'CMP-TEST-WARD'"),
            },
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: "wardId IN (SELECT id FROM WardEntity WHERE wardName = 'CMP-TEST-WARD')".to_string(),
            },
        ],
        &["id", "createTime", "updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_ward_post_add_by_dept_id() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let dept_id = test_dept_id();
    let body = serde_json::json!({
        "ward": {
            "wardName": "CMP-DEPT-WARD",
            "wardType": "普通",
            "effectiveness": 1
        },
        "sickbeds": []
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "ward.addByDeptId",
        &format!("{}/ward/addByDeptId?departmentId={}", java_base_url(), dept_id),
        &format!("{}/ward/addByDeptId?departmentId={}", rust_base_url(), dept_id),
        Some(&body),
        &[
            AffectedTable {
                table: "WardEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("wardName = 'CMP-DEPT-WARD'"),
            },
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: "wardId IN (SELECT id FROM WardEntity WHERE wardName = 'CMP-DEPT-WARD')".to_string(),
            },
        ],
        &["id", "createTime", "updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_ward_post_update() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "ward": {
            "id": test_ward_id(),
            "wardName": "CMP-UPD-WARD",
            "wardType": "普通",
            "departmentId": test_dept_id(),
            "effectiveness": 1
        },
        "sickbeds": []
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "ward.update",
        &format!("{}/ward/update", java_base_url()),
        &format!("{}/ward/update", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "WardEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_ward_id()),
            },
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("wardId = '{}'", test_ward_id()),
            },
        ],
        &["updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_ward_post_delete() {
    let client = http_client();
    let rb = init_test_rbatis().await;

    test_mutation_with_db_diff(
        &client,
        &rb,
        "ward.delete",
        &format!("{}/ward/delete?id={}", java_base_url(), "nonexistent-ward-for-compare"),
        &format!("{}/ward/delete?id={}", rust_base_url(), "nonexistent-ward-for-compare"),
        None,
        &[
            AffectedTable {
                table: "WardEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = 'nonexistent-ward-for-compare'"),
            },
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: "wardId = 'nonexistent-ward-for-compare'".to_string(),
            },
        ],
        &[],
    ).await;
}

#[tokio::test]
async fn compare_ward_post_find_by_ward_id() {
    let client = http_client();
    let body = serde_json::json!({
        "departmentId": test_dept_id(),
        "wardName": "",
        "wardType": ""
    });

    let mut java_resp = client
        .post(format!("{}/ward/findbywardId", java_base_url()))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    let mut rust_resp = client
        .post(format!("{}/ward/findbywardId", rust_base_url()))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    sort_json_arrays(&mut java_resp, "id");
    sort_json_arrays(&mut rust_resp, "id");

    let diffs = deep_diff("ward.findbywardId", &java_resp, &rust_resp);
    assert_no_significant_diffs("ward.findbywardId", &diffs);
}
