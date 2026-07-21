//! Sickbed 业务域接口对比测试
//!
//! 运行方式：
//! ```bash
//! JAVA_BASE_URL=http://localhost:8081 RUST_BASE_URL=http://localhost:8080 \
//!   cargo test -p sickbed --test sickbed_comparison_tests -- --nocapture
//! ```

mod common;

use common::*;
use serde_json::Value;

// ==================== Sickbed GET 端点对比 ====================

#[tokio::test]
async fn compare_sickbed_get_find_by_id() {
    let client = http_client();
    let id = test_sickbed_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!("{}/id/{}", java_base_url(), id))
            .send(),
        client
            .get(format!("{}/id/{}", rust_base_url(), id))
            .send(),
    );

    let java_text = java_resp.unwrap().text().await.unwrap();
    let java_json: Value = serde_json::from_str(&java_text)
        .unwrap_or_else(|e| panic!("Java 响应 JSON 解析失败: {}, 响应体: '{}'", e, java_text));
    let rust_text = rust_resp.unwrap().text().await.unwrap();
    let rust_json: Value = serde_json::from_str(&rust_text)
        .unwrap_or_else(|e| panic!("Rust 响应 JSON 解析失败: {}, 响应体: '{}'", e, rust_text));

    let diffs = deep_diff("sickbed.findById", &java_json, &rust_json);
    assert_no_diffs("sickbed.findById", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_effectiveness() {
    let client = http_client();
    let dept_id = test_dept_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/effectiveness?departmentId={}",
                java_base_url(),
                dept_id
            ))
            .send(),
        client
            .get(format!(
                "{}/effectiveness?departmentId={}",
                rust_base_url(),
                dept_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.effectiveness", &java_json, &rust_json);
    assert_no_diffs("sickbed.effectiveness", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_effectiveness_for_search() {
    let client = http_client();
    let dept_id = test_dept_id();
    let qs = format!(
        "departmentId={}&wardId=&sickbedNo=&pageIndex=0&pageSize=10",
        dept_id
    );

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/effectivenessForSearch?{}",
                java_base_url(),
                qs
            ))
            .send(),
        client
            .get(format!(
                "{}/effectivenessForSearch?{}",
                rust_base_url(),
                qs
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.effectivenessForSearch", &java_json, &rust_json);
    assert_no_diffs("sickbed.effectivenessForSearch", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_bedside_effectiveness() {
    let client = http_client();
    let dept_id = test_dept_id();

    // 清理测试脏数据，确保数据一致
    let rb = init_test_rbatis().await;
    cleanup_test_artifacts(&rb).await;

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/bedside/effectiveness?departmentId={}",
                java_base_url(),
                dept_id
            ))
            .send(),
        client
            .get(format!(
                "{}/bedside/effectiveness?departmentId={}",
                rust_base_url(),
                dept_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.bedsideEffectiveness", &java_json, &rust_json);
    assert_no_diffs("sickbed.bedsideEffectiveness", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_effectiveness_empty() {
    let client = http_client();
    let dept_id = test_dept_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/effectiveness/empty?departmentId={}",
                java_base_url(),
                dept_id
            ))
            .send(),
        client
            .get(format!(
                "{}/effectiveness/empty?departmentId={}",
                rust_base_url(),
                dept_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.effectivenessEmpty", &java_json, &rust_json);
    assert_no_diffs("sickbed.effectivenessEmpty", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_effectiveness_usersickbed() {
    let client = http_client();
    let dept_id = test_dept_id();
    let user_id = test_user_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/effectiveness/usersickbed?departmentId={}&userId={}",
                java_base_url(),
                dept_id,
                user_id
            ))
            .send(),
        client
            .get(format!(
                "{}/effectiveness/usersickbed?departmentId={}&userId={}",
                rust_base_url(),
                dept_id,
                user_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.effectivenessUsersickbed", &java_json, &rust_json);
    assert_no_diffs("sickbed.effectivenessUsersickbed", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_effectivenesse_department_count() {
    let client = http_client();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/effectivenessedepartmentcount",
                java_base_url()
            ))
            .send(),
        client
            .get(format!(
                "{}/effectivenessedepartmentcount",
                rust_base_url()
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff(
        "sickbed.effectivenesseDepartmentCount",
        &java_json,
        &rust_json,
    );
    assert_no_diffs("sickbed.effectivenesseDepartmentCount", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_effectiveness_count() {
    let client = http_client();
    let dept_id = test_dept_id();

    // 清理测试脏数据，确保数据一致
    let rb = init_test_rbatis().await;
    cleanup_test_artifacts(&rb).await;

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/effectivenesscount?departmentId={}",
                java_base_url(),
                dept_id
            ))
            .send(),
        client
            .get(format!(
                "{}/effectivenesscount?departmentId={}",
                rust_base_url(),
                dept_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.effectivenessCount", &java_json, &rust_json);
    assert_no_diffs("sickbed.effectivenessCount", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_find_by_scan_no() {
    let client = http_client();
    let scan_no = test_scan_no();

    // 清理测试脏数据，确保数据一致
    let rb = init_test_rbatis().await;
    cleanup_test_artifacts(&rb).await;

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/findbyscanno?scanNo={}",
                java_base_url(),
                scan_no
            ))
            .send(),
        client
            .get(format!(
                "{}/findbyscanno?scanNo={}",
                rust_base_url(),
                scan_no
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.findByScanNo", &java_json, &rust_json);
    assert_no_diffs("sickbed.findByScanNo", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_find_by_ward_id() {
    let client = http_client();
    let ward_id = test_ward_id();

    // 清理测试脏数据，确保数据一致
    let rb = init_test_rbatis().await;
    cleanup_test_artifacts(&rb).await;

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/findbywardId?wardId={}",
                java_base_url(),
                ward_id
            ))
            .send(),
        client
            .get(format!(
                "{}/findbywardId?wardId={}",
                rust_base_url(),
                ward_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.findByWardId", &java_json, &rust_json);
    assert_no_diffs("sickbed.findByWardId", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_find_sickbed_info_by_ward_id() {
    let client = http_client();
    let ward_id = test_ward_id();

    // 清理测试脏数据，确保数据一致
    let rb = init_test_rbatis().await;
    cleanup_test_artifacts(&rb).await;

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/findSickbedInfoByWardId?wardId={}",
                java_base_url(),
                ward_id
            ))
            .send(),
        client
            .get(format!(
                "{}/findSickbedInfoByWardId?wardId={}",
                rust_base_url(),
                ward_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.findSickbedInfoByWardId", &java_json, &rust_json);
    assert_no_diffs("sickbed.findSickbedInfoByWardId", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_find_ward_sickbed_id_by_sickbed_id() {
    let client = http_client();
    let sickbed_id = test_sickbed_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/findWardSickbedIdBySickbedId?sickbedId={}",
                java_base_url(),
                sickbed_id
            ))
            .send(),
        client
            .get(format!(
                "{}/findWardSickbedIdBySickbedId?sickbedId={}",
                rust_base_url(),
                sickbed_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff(
        "sickbed.findWardSickbedIdBySickbedId",
        &java_json,
        &rust_json,
    );
    assert_no_diffs("sickbed.findWardSickbedIdBySickbedId", &diffs);
}

#[tokio::test]
async fn compare_sickbed_get_find_patient_id_by_ward_id() {
    let client = http_client();
    let ward_id = test_ward_id();

    let (java_resp, rust_resp) = tokio::join!(
        client
            .get(format!(
                "{}/findPatientIdByWardId?wardId={}",
                java_base_url(),
                ward_id
            ))
            .send(),
        client
            .get(format!(
                "{}/findPatientIdByWardId?wardId={}",
                rust_base_url(),
                ward_id
            ))
            .send(),
    );

    let mut java_json: Value = java_resp.unwrap().json().await.unwrap();
    let mut rust_json: Value = rust_resp.unwrap().json().await.unwrap();

    sort_json_arrays(&mut java_json, "id");
    sort_json_arrays(&mut rust_json, "id");

    let diffs = deep_diff("sickbed.findPatientIdByWardId", &java_json, &rust_json);
    assert_no_diffs("sickbed.findPatientIdByWardId", &diffs);
}

// ==================== Sickbed POST 端点对比 ====================

#[tokio::test]
async fn compare_sickbed_post_add() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "sickbedNo": "CMP-TEST-001",
        "wardId": test_ward_id(),
        "departmentId": test_dept_id(),
        "effectiveness": 1,
        "sickbedType": "普通",
        "groupNo": "1"
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.add",
        &format!("{}/add", java_base_url()),
        &format!("{}/add", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("sickbedNo = 'CMP-TEST-001'"),
            },
            AffectedTable {
                table: "WardEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_ward_id()),
            },
        ],
        &["id", "createTime", "updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_update() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "id": test_sickbed_id(),
        "sickbedNo": "CMP-UPD-001",
        "wardId": test_ward_id(),
        "departmentId": test_dept_id(),
        "effectiveness": 1,
        "sickbedType": "普通",
        "groupNo": "2"
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.update",
        &format!("{}/update", java_base_url()),
        &format!("{}/update", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_sickbed_id()),
            },
            AffectedTable {
                table: "message",
                pk_field: "id",
                order_by: "creation_time",
                where_clause: "destination LIKE '%SickbedUpdatedEvent%'".to_string(),
            },
        ],
        &["updateTime", "id", "creation_time", "headers", "published"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_delete() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!(["nonexistent-id-for-compare"]);

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.delete",
        &format!("{}/delete", java_base_url()),
        &format!("{}/delete", rust_base_url()),
        Some(&body),
        &[AffectedTable {
            table: "SickbedEntity",
            pk_field: "id",
            order_by: "id",
            where_clause: format!("id = 'nonexistent-id-for-compare'"),
        }],
        &[],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_batchupdate() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!([{
        "id": test_sickbed_id(),
        "sickbedNo": "CMP-BATCH-001",
        "wardId": test_ward_id(),
        "departmentId": test_dept_id(),
        "effectiveness": 1,
        "groupNo": "1"
    }]);

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.batchupdate",
        &format!("{}/batchupdate", java_base_url()),
        &format!("{}/batchupdate", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_sickbed_id()),
            },
            AffectedTable {
                table: "message",
                pk_field: "id",
                order_by: "creation_time",
                where_clause: "destination LIKE '%SickbedUpdatedEvent%'".to_string(),
            },
        ],
        &["updateTime", "id", "creation_time", "headers", "published"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_arrangement() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "sickbedId": test_sickbed_id(),
        "patientId": "cmp-patient-001",
        "departmentId": test_dept_id()
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.arrangement",
        &format!("{}/arrangement", java_base_url()),
        &format!("{}/arrangement", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_sickbed_id()),
            },
            AffectedTable {
                table: "message",
                pk_field: "id",
                order_by: "creation_time",
                where_clause: "destination LIKE '%SickbedArrangementedEvent%'".to_string(),
            },
        ],
        &["updateTime", "id", "creation_time", "headers", "published"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_change() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "sourceSickbedId": test_sickbed_id(),
        "targetSickbedId": "target-sickbed-id",
        "patientId": "cmp-patient-001",
        "departmentId": test_dept_id()
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.change",
        &format!("{}/change", java_base_url()),
        &format!("{}/change", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id IN ('{}', 'target-sickbed-id')", test_sickbed_id()),
            },
            AffectedTable {
                table: "message",
                pk_field: "id",
                order_by: "creation_time",
                where_clause: "destination LIKE '%sickbed%'".to_string(),
            },
        ],
        &["updateTime", "id", "creation_time", "headers", "published"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_empty() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "sickbedId": test_sickbed_id(),
        "departmentId": test_dept_id()
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.empty",
        &format!("{}/empty", java_base_url()),
        &format!("{}/empty", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_sickbed_id()),
            },
            AffectedTable {
                table: "message",
                pk_field: "id",
                order_by: "creation_time",
                where_clause: "destination LIKE '%SickbedEmptyedEvent%'".to_string(),
            },
        ],
        &["updateTime", "id", "creation_time", "headers", "published"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_testarrangement() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let body = serde_json::json!({
        "sickbedId": test_sickbed_id(),
        "patientId": "cmp-patient-test-001",
        "departmentId": test_dept_id()
    });

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.testarrangement",
        &format!("{}/testarrangement", java_base_url()),
        &format!("{}/testarrangement", rust_base_url()),
        Some(&body),
        &[
            AffectedTable {
                table: "SickbedEntity",
                pk_field: "id",
                order_by: "id",
                where_clause: format!("id = '{}'", test_sickbed_id()),
            },
            AffectedTable {
                table: "message",
                pk_field: "id",
                order_by: "creation_time",
                where_clause: "destination LIKE '%SickbedTestArrangementedEvent%'".to_string(),
            },
        ],
        &["updateTime", "id", "creation_time", "headers", "published"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_forceemptyall() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let dept_code = test_dept_id();

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.forceemptyall",
        &format!("{}/forceemptyall?departmentCode={}", java_base_url(), dept_code),
        &format!("{}/forceemptyall?departmentCode={}", rust_base_url(), dept_code),
        None,
        &[AffectedTable {
            table: "SickbedEntity",
            pk_field: "id",
            order_by: "id",
            where_clause: format!("departmentId = '{}'", test_dept_id()),
        }],
        &["updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_forceemptyone() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let dept_code = test_dept_id();
    let sickbed_no = "001";

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.forceemptyone",
        &format!("{}/forceemptyone?departmentCode={}&sickbedNo={}", java_base_url(), dept_code, sickbed_no),
        &format!("{}/forceemptyone?departmentCode={}&sickbedNo={}", rust_base_url(), dept_code, sickbed_no),
        None,
        &[AffectedTable {
            table: "SickbedEntity",
            pk_field: "id",
            order_by: "id",
            where_clause: format!("departmentId = '{}'", dept_code),
        }],
        &["updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_emptynurse() {
    let client = http_client();
    let rb = init_test_rbatis().await;
    let dept_id = test_dept_id();
    let user_id = test_user_id();

    test_mutation_with_db_diff(
        &client,
        &rb,
        "sickbed.emptynurse",
        &format!("{}/emptynurse?departmentId={}&userId={}", java_base_url(), dept_id, user_id),
        &format!("{}/emptynurse?departmentId={}&userId={}", rust_base_url(), dept_id, user_id),
        None,
        &[AffectedTable {
            table: "SickbedEntity",
            pk_field: "id",
            order_by: "id",
            where_clause: format!("departmentId = '{}'", dept_id),
        }],
        &["updateTime"],
    ).await;
}

#[tokio::test]
async fn compare_sickbed_post_effectivenesscountbylist() {
    let client = http_client();
    let body = serde_json::json!([test_dept_id()]);

    let mut java_resp = client
        .post(format!("{}/effectivenesscountbylist", java_base_url()))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    let mut rust_resp = client
        .post(format!("{}/effectivenesscountbylist", rust_base_url()))
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    sort_json_arrays(&mut java_resp, "id");
    sort_json_arrays(&mut rust_resp, "id");

    let diffs = deep_diff("sickbed.effectivenesscountbylist", &java_resp, &rust_resp);
    assert_no_significant_diffs("sickbed.effectivenesscountbylist", &diffs);
}

#[tokio::test]
async fn compare_sickbed_post_get_all_bed() {
    let client = http_client();

    let mut java_resp = client
        .post(format!("{}/getAllBed", java_base_url()))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    let mut rust_resp = client
        .post(format!("{}/getAllBed", rust_base_url()))
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    sort_json_arrays(&mut java_resp, "id");
    sort_json_arrays(&mut rust_resp, "id");

    let diffs = deep_diff("sickbed.getAllBed", &java_resp, &rust_resp);
    assert_no_significant_diffs("sickbed.getAllBed", &diffs);
}
