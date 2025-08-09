use std::sync::Arc;
use salvo::prelude::*;
use serde::{Serialize, Deserialize};
use casbin::{CoreApi, Enforcer};
use genies_derive::casbin;

#[casbin]
#[derive( Deserialize,ToSchema)]
pub struct User {
    pub id: u64,
    pub name: Option<String>,
    pub email: String,
    pub phone: String,
    pub credit_card: String,
}

#[endpoint]
async fn get_user(req: &mut Request, res: &mut Response, depot: &mut Depot) ->Json<User> {
    let enforcer = depot.obtain::<Arc<Enforcer>>().unwrap();
    let user = req.query::<String>("user").unwrap_or("guest".into());

    let p = User {
        id: 1,
        name: Some("张三".to_string()),
        email: "zhangsan@example.com".to_string(),
        phone: "13800138000".to_string(),
        credit_card: "1234-5678-9012-3456".to_string(),
        enforcer: None,
        subject: None,
    };

    // 应用策略
    let user_with_policy = p.with_policy(Arc::clone(&enforcer), user);

    // 返回 JSON 响应
    Json(user_with_policy)

}

#[tokio::main]
async fn main() {
    let enforcer = Enforcer::new("model.conf", "policy.csv").await.unwrap();

    let router = Router::new()
        .hoop(affix_state::inject(Arc::new(enforcer)))
        .get(get_user);
    let doc = OpenApi::new("casbin", "0.0.1").merge_router(&router);
    let router = router
        .unshift(doc.into_router("/api-doc/openapi.json"))
        .unshift(SwaggerUi::new("/api-doc/openapi.json")
            .into_router("/swagger-ui"));
    let acceptor = TcpListener::new("0.0.0.0:5800").bind().await;
    Server::new(acceptor).serve(router).await;
}

