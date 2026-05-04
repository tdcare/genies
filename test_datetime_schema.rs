use salvo::oapi::ToSchema;
use serde::{Deserialize, Serialize};
use rbdc::DateTime;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TestDateTime {
    pub id: i64,
    pub created_at: Option<DateTime>,
    pub updated_at: Option<String>,
    pub count: Option<i64>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_print_schema() {
        let schema = TestDateTime::schema();
        let json = serde_json::to_value(&schema).unwrap();
        println!('{}', serde_json::to_string_pretty(&json).unwrap());
    }
}
