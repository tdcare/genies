//! Patient 微服务远程调用
//!
//! 对应 Java: me.tdcarefor.tdnis.patient.remote.PatientInfo (FeignClient)
//! 服务路由前缀: /patient

use genies_derive::remote;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// Patient 微服务基础路径
pub static PATIENT: Lazy<String> = genies::config_gateway!("/patient");

// ============================================================
// 返回结构体
// ============================================================

/// 患者模型（远程调用返回）
///
/// 对应 Java: me.tdcarefor.tdnis.patient.model.PatientModel
/// 仅包含 sickbed 服务实际使用到的字段
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PatientModel {
    pub id: Option<String>,
    pub name: Option<String>,
    pub sex: Option<i32>,
    pub age: Option<String>,
    pub patient_no: Option<String>,
    pub department_id: Option<String>,
    pub department_code: Option<String>,
    pub his_id: Option<String>,
    pub sickbed_id: Option<String>,
    pub sickbed_no: Option<String>,
    pub doctor_user_id: Option<String>,
    pub doctor_user_name: Option<String>,
    pub status: Option<i32>,
    pub is_noneatal: Option<i32>,
    pub pre_out_hospital: Option<i32>,
}

// ============================================================
// 远程调用函数
// ============================================================

// --- 通过腕带扫描号查询患者 ---
// 对应 Java: PatientInfo.findByScanNo(printId)
// GET /scanno?print_id=xxx

/// 通过腕带扫描号查询患者（自动 token 管理）
#[remote]
#[get(url = PATIENT, path = "/scanno")]
pub async fn find_patient_by_scan_no(
    #[query] print_id: &str,
) -> feignhttp_rs::Result<PatientModel> { impled!() }

// --- 通过患者ID查询在院患者（含医生信息）---
// 对应 Java: PatientInfo.inFindById(patientId)
// GET /infindbyid?patient_id=xxx

/// 通过患者ID查询在院患者（自动 token 管理）
#[remote]
#[get(url = PATIENT, path = "/infindbyid")]
pub async fn find_in_patient_by_id(
    #[query] patient_id: &str,
) -> feignhttp_rs::Result<PatientModel> { impled!() }

// --- 通过床位ID列表批量查询患者 ---
// 对应 Java: PatientInfo.findbysickbedid(List<String>)
// POST /findbysickbedid body=List<String>

/// 通过床位ID列表批量查询患者（自动 token 管理）
#[remote]
#[post(url = PATIENT, path = "/findbysickbedid")]
pub async fn find_patients_by_sickbed_ids(
    #[body] sickbed_ids: &Vec<String>,
) -> feignhttp_rs::Result<Vec<PatientModel>> { impled!() }

// --- 通过床位ID列表获取在院患者门口屏信息 ---
// 对应 Java: PatientInfo.findInHostbySickbedIds(List<String>)
// GET /findInHostbySickbedIds?sickbedId=xxx&sickbedId=yyy
// 因为 feignhttp 不支持重复 query 参数，此处手写 HTTP 调用

/// 通过床位ID列表获取在院患者门口屏信息（自动 token 管理）
///
/// 返回 patient 服务的 ResultDTO 原样透传。
pub async fn find_in_host_by_sickbed_ids(
    sickbed_ids: &[String],
) -> Result<genies::core::ResultDTO<Vec<crate::model::vo::PatientDoorwayScreenVo>>, String> {
    use reqwest::StatusCode;

    let base_url = &*PATIENT;
    let query_string: String = sickbed_ids
        .iter()
        .map(|id| format!("sickbedId={}", id))
        .collect::<Vec<_>>()
        .join("&");
    let url = if query_string.is_empty() {
        format!("{}/findInHostbySickbedIds", base_url)
    } else {
        format!("{}/findInHostbySickbedIds?{}", base_url, query_string)
    };

    let client = reqwest::Client::builder().no_proxy().build().map_err(|e| e.to_string())?;

    // 首次尝试：优先使用请求级用户 token，降级使用 REMOTE_TOKEN
    let bearer = genies::context::request_token::get_request_token()
        .unwrap_or_else(|| {
            let token = genies::context::REMOTE_TOKEN.lock().unwrap();
            format!("Bearer {}", &token.access_token)
        });

    let resp = client
        .get(&url)
        .header("Authorization", &bearer)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status() == StatusCode::UNAUTHORIZED {
        // 刷新 token 后重试
        if let Ok(new_token) = genies::core::jwt::get_temp_access_token(
            &genies::context::CONTEXT.config.keycloak_auth_server_url,
            &genies::context::CONTEXT.config.keycloak_realm,
            &genies::context::CONTEXT.config.keycloak_resource,
            &genies::context::CONTEXT.config.keycloak_credentials_secret,
        )
        .await
        {
            genies::context::REMOTE_TOKEN.lock().unwrap().access_token = new_token.clone();
            let bearer2 = format!("Bearer {}", &new_token);
            let resp2 = client
                .get(&url)
                .header("Authorization", &bearer2)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            return resp2.json().await.map_err(|e| e.to_string());
        }
    }

    resp.json().await.map_err(|e| e.to_string())
}
