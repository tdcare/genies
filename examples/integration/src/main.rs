//! 集成测试服务器
//!
//! 启动方式: `cargo run -p integration`
//! 访问 Auth UI: http://127.0.0.1:18080/auth/ui/

use genies::context::CONTEXT;
use genies_auth::{
    auth_admin_router, auth_admin_ui_router, auth_public_router, casbin_auth, extract_and_sync_schemas,
    EnforcerManager,
};
use genies_derive::casbin;
use salvo::oapi::swagger_ui::SwaggerUi;
use salvo::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ==================== 嵌套结构体定义 ====================

/// 地址信息
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Address {
    /// 省份
    pub province: String,
    /// 城市
    pub city: String,
    /// 区/县
    pub district: String,
    /// 详细地址
    pub street: String,
    /// 邮政编码
    pub postal_code: String,
    /// 是否为默认地址
    pub is_default: bool,
}

/// 联系方式
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ContactInfo {
    /// 手机号码
    pub mobile: String,
    /// 固定电话
    pub telephone: Option<String>,
    /// 电子邮箱
    pub email: String,
    /// 微信号
    pub wechat: Option<String>,
    /// 紧急联系人姓名
    pub emergency_contact_name: Option<String>,
    /// 紧急联系人电话
    pub emergency_contact_phone: Option<String>,
}

/// 银行账户信息
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BankAccount {
    /// 开户银行名称
    pub bank_name: String,
    /// 银行账号
    pub account_number: String,
    /// 开户行支行
    pub branch_name: String,
    /// 账户类型：储蓄卡/信用卡
    pub account_type: String,
}

/// 工作经历
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct WorkExperience {
    /// 公司名称
    pub company_name: String,
    /// 职位
    pub position: String,
    /// 入职年份
    pub start_year: i32,
    /// 离职年份（在职则为空）
    pub end_year: Option<i32>,
    /// 月薪（元）
    pub monthly_salary: f64,
    /// 工作描述
    pub description: Option<String>,
}

/// 技能标签
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct Skill {
    /// 技能名称
    pub name: String,
    /// 熟练程度（1-5）
    pub proficiency_level: i32,
    /// 使用年限
    pub years_of_experience: f64,
}

// ==================== 主结构体定义 ====================

/// 员工信息对象
/// 
/// 包含员工的完整档案信息，用于测试 casbin 字段级权限控制
#[casbin]
#[derive(Serialize, Deserialize, ToSchema)]
pub struct Employee {
    /// 员工唯一标识（主键）
    pub id: u64,
    /// 工号
    pub employee_number: String,
    /// 员工姓名
    pub name: String,
    /// 性别（true=男，false=女）
    pub gender: bool,
    /// 年龄
    pub age: i32,
    /// 身份证号码（敏感信息）
    pub id_card_number: String,
    /// 基本工资（敏感信息）
    pub base_salary: f64,
    /// 绩效奖金比例
    pub bonus_rate: f64,
    /// 入职日期（时间戳）
    pub hire_date: u64,
    /// 是否在职
    pub is_active: bool,
    /// 部门ID
    pub department_id: Option<i32>,
    /// 直属上级工号
    pub manager_employee_number: Option<String>,
    /// 职级（1-10）
    pub job_level: i32,
    /// 家庭住址
    pub home_address: Address,
    /// 工作地址
    pub work_address: Option<Address>,
    /// 联系方式
    pub contact: ContactInfo,
    /// 银行账户（敏感信息）
    pub bank_accounts: Vec<BankAccount>,
    /// 工作经历
    pub work_experiences: Vec<WorkExperience>,
    /// 技能标签
    pub skills: Vec<Skill>,
    /// 兴趣爱好
    pub hobbies: Vec<String>,
    /// 备注
    pub notes: Option<String>,
}

/// 获取员工信息
#[endpoint]
async fn get_user() -> Json<Employee> {
    Json(Employee {
        id: 10001,
        employee_number: "EMP-2024-001".into(),
        name: "张三".into(),
        gender: true,
        age: 28,
        id_card_number: "310101199501011234".into(),
        base_salary: 25000.50,
        bonus_rate: 0.15,
        hire_date: 1704067200, // 2024-01-01
        is_active: true,
        department_id: Some(101),
        manager_employee_number: Some("EMP-2020-088".into()),
        job_level: 5,
        home_address: Address {
            province: "上海市".into(),
            city: "上海市".into(),
            district: "浦东新区".into(),
            street: "张江高科技园区碧波路888号".into(),
            postal_code: "201203".into(),
            is_default: true,
        },
        work_address: Some(Address {
            province: "上海市".into(),
            city: "上海市".into(),
            district: "徐汇区".into(),
            street: "漕河泾开发区田林路888号".into(),
            postal_code: "200233".into(),
            is_default: false,
        }),
        contact: ContactInfo {
            mobile: "13800138000".into(),
            telephone: Some("021-12345678".into()),
            email: "zhangsan@example.com".into(),
            wechat: Some("zhangsan_wx".into()),
            emergency_contact_name: Some("李四".into()),
            emergency_contact_phone: Some("13900139000".into()),
        },
        bank_accounts: vec![
            BankAccount {
                bank_name: "中国工商银行".into(),
                account_number: "6222021234567890123".into(),
                branch_name: "上海市浦东新区张江支行".into(),
                account_type: "储蓄卡".into(),
            },
            BankAccount {
                bank_name: "招商银行".into(),
                account_number: "6225881234567890".into(),
                branch_name: "上海市徐汇区漕河泾支行".into(),
                account_type: "信用卡".into(),
            },
        ],
        work_experiences: vec![
            WorkExperience {
                company_name: "阿里巴巴".into(),
                position: "高级工程师".into(),
                start_year: 2020,
                end_year: Some(2023),
                monthly_salary: 35000.0,
                description: Some("负责电商平台后端架构设计与开发".into()),
            },
            WorkExperience {
                company_name: "字节跳动".into(),
                position: "技术专家".into(),
                start_year: 2024,
                end_year: None,
                monthly_salary: 50000.0,
                description: Some("负责推荐系统核心算法优化".into()),
            },
        ],
        skills: vec![
            Skill {
                name: "Rust".into(),
                proficiency_level: 4,
                years_of_experience: 3.5,
            },
            Skill {
                name: "Go".into(),
                proficiency_level: 5,
                years_of_experience: 5.0,
            },
            Skill {
                name: "Kubernetes".into(),
                proficiency_level: 4,
                years_of_experience: 4.0,
            },
        ],
        hobbies: vec![
            "篮球".into(),
            "摄影".into(),
            "旅行".into(),
            "阅读".into(),
        ],
        notes: Some("优秀员工，2023年度最佳贡献奖获得者".into()),
    })
}

#[tokio::main]
async fn main() {
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n========================================");
    println!("    Auth 集成测试服务器");
    println!("========================================\n");

    // 初始化 MySQL
    println!("[步骤 1/5] 初始化 MySQL 连接...");
    CONTEXT.init_mysql().await;
    println!("[步骤 1/5] MySQL 连接成功\n");

    // 运行数据库迁移
    println!("[步骤 2/5] 运行数据库迁移...");
    genies_auth::models::run_migrations().await;
    println!("[步骤 2/5] 数据库迁移完成\n");

    // 构建业务路由
    println!("[步骤 3/5] 配置路由...");
    let business_router = Router::new().push(Router::with_path("/api/users").get(get_user));

    // OpenApi Schema 同步（合并所有需要文档化的路由）
    let auth_admin = auth_admin_router();
    let doc = OpenApi::new("auth-service", "1.0.0")
        .merge_router(&business_router)
        .merge_router(&auth_admin);
    if let Err(e) = extract_and_sync_schemas(&doc).await {
        println!("[警告] Schema 同步失败: {}", e);
    }

    // 创建 Enforcer
    println!("[步骤 4/5] 初始化 Casbin Enforcer...");
    let mgr = Arc::new(
        EnforcerManager::new()
            .await
            .expect("Enforcer 初始化失败"),
    );
    println!("[步骤 4/5] Enforcer 初始化完成\n");

    // 挂载中间件到业务路由
    let protected_router = business_router
        .hoop(genies::context::auth::salvo_auth)
        .hoop(affix_state::inject(mgr.clone()))
        .hoop(casbin_auth)
        .push(auth_admin);

    // 构建顶层路由
    let router = Router::new()
        .push(protected_router)
        .push(auth_admin_ui_router()) // 静态资源不需要认证
        .push(auth_public_router())   // Token 端点不需要认证
        .push(genies::k8s::k8s_health_check())
        // OpenAPI 文档（不需要认证）
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(SwaggerUi::new("/api-doc/openapi.json").into_router("/swagger-ui"));

    // 启动服务器
    println!("[步骤 5/5] 启动 HTTP 服务器...");
    let acceptor = TcpListener::new("127.0.0.1:18080").bind().await;

    println!("\n========================================");
    println!("    服务器启动成功!");
    println!("========================================");
    println!("  端口: 18080");
    println!("  Auth UI: http://127.0.0.1:18080/auth/ui/");
    println!("  Admin API: http://127.0.0.1:18080/auth/");
    println!("  API 文档: http://127.0.0.1:18080/swagger-ui/");
    println!("  OpenAPI JSON: http://127.0.0.1:18080/api-doc/openapi.json");
    println!("  健康检查: http://127.0.0.1:18080/health");
    println!("========================================");
    println!("  按 Ctrl+C 停止服务器");
    println!("========================================\n");

    Server::new(acceptor).serve(router).await;
}
