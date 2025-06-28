/*
 * @Author: tzw
 * @Date: 2021-07-21 22:45:16
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-09 21:12:57
 */
use crate::error::Error;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;


/// JWT 鉴权 Token结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JWTToken {
    //账号id
    pub id:  Option<String>,
    //账号
    // pub account: String,
    // //权限集合
    // pub permissions: Vec<String>,
    // //角色id集合
    // pub role_ids: Vec<String>,
    //过期时间
    pub exp: Option<usize>,
    pub iat: Option<usize>,
    pub jti: Option<String>,
    pub iss: Option<String>,
    pub sub: Option<String>,
    pub typ: Option<String>,
    pub azp: Option<String>,
    pub session_state: Option<String>,
    pub acr: Option<String>,
    pub realm_access: Option<Value>,
    pub resource_access: Option<Value>,
    // // "realm_access": {
    // // 	"roles": ["offline_access", "nurse", "uma_authorization", "user"]
    // // },
    // // "resource_access": {
    // // 	"tdnis": {
    // // 		"roles": ["nurseManager", "user"]
    // // 	}
    // // },
    pub scope: Option<String>,
    #[serde(rename = "departmentName")]
    pub department_name: Option<String>,
    // pub address: String,
    #[serde(rename = "departmentCode")]
    pub department_code: Option<String>,
    #[serde(rename = "departmentId")]
    pub department_id: Option<String>,
    pub roles: Option<Vec<String>>,
    pub groups: Option<Vec<String>>,
    pub dept: Option<Vec<String>>,
    pub preferred_username: Option<String>,
    pub given_name: Option<String>,
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "departmentAbstract")]
    pub department_abstract: Option<String>,
}

impl JWTToken {
    /// create token
    /// secret: your secret string
    pub fn create_token(&self, secret: &str) -> Result<String,  crate::error::Error> {
        return match encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret.as_ref()),
        ) {
            Ok(t) => Ok(t),
            Err(_) => Err(Error::from("JWTToken encode fail!")), // in practice you would return the error
        };
    }
    /// verify token invalid
    /// secret: your secret string
    pub fn verify(secret: &str, token: &str) -> Result<JWTToken,  crate::error::Error> {
        let validation = Validation {
            ..Validation::default()
        };
        return match decode::<JWTToken>(
            &token,
            &DecodingKey::from_secret(secret.as_ref()),
            &validation,
        ) {
            Ok(c) => Ok(c.claims),
            Err(err) => match *err.kind() {
                ErrorKind::InvalidToken => return Err(Error::from("InvalidToken")), // Example on how to handle a specific error
                ErrorKind::InvalidIssuer => return Err(Error::from("InvalidIssuer")), // Example on how to handle a specific error
                _ => return Err(Error::from("InvalidToken other errors")),
            },
        };
    }
 // 从keycloak 服务中读取相关的加密算法，和相关的key 然后进行解密验证
    pub fn verify_with_keycloak(keycloak: &Keys, token: &str) -> Result<JWTToken,  crate::error::Error> {
        let n = keycloak.keys[0].n.as_ref().unwrap();
        let e = keycloak.keys[0].e.as_ref().unwrap();
        // let kty = keycloak.keys[0].kty.as_ref().unwrap();
        let alg = keycloak.keys[0].alg.as_ref().unwrap();

        let algorithm = Algorithm::from_str(&alg).unwrap();

        let validation = Validation::new(algorithm);

        let jwt_token =
            decode::<JWTToken>(&token, &DecodingKey::from_rsa_components(n, e), &validation);

        return match jwt_token {
            Ok(c) => Ok(c.claims),
            Err(err) => match *err.kind() {
                ErrorKind::InvalidToken => return Err(Error::from("InvalidToken")), // Example on how to handle a specific error
                ErrorKind::InvalidIssuer => return Err(Error::from("InvalidIssuer")), // Example on how to handle a specific error
                _ => return Err(Error::from("InvalidToken other errors")),
            },
        };
    }
}

///keycloak 秘钥
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Keys {
    pub keys: Vec<Key>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Key {
    pub kid: Option<String>,
    pub kty: Option<String>,
    pub alg: Option<String>,
    #[serde(rename = "use")]
    pub r#use: Option<String>,
    pub n: Option<String>,
    pub e: Option<String>,
    pub x5c: Option<Vec<String>>,
    pub x5t: Option<String>,
    #[serde(rename = "x5t#S256")]
    pub x5t_s256: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyCloakAccessToken {
    pub access_token: Option<String>,
    pub expires_in: Option<i64>,
    pub refresh_expires_in: Option<i64>,
    pub refresh_token: Option<String>,
    pub token_type: Option<String>,
    // pub not-before-polic:Option<String>,
    pub session_state: Option<String>,
    pub scope: Option<String>,
}

pub async fn get_keycloak_keys(keycloak_auth_server_url: &str, keycloak_realm: &str) -> Keys {
    // let keycloak_auth_server_url=&config.keycloak_auth_server_url;
    // let keycloak_realm=&config.keycloak_realm;
    let c = format!(
        "{}realms/{}/protocol/openid-connect/certs",
        keycloak_auth_server_url, keycloak_realm
    );
    log::info!("开始访问keycloak:{}",c);

    // use hyper::*;
    // let client = Client::new();
    // let res = client.get(<Uri as std::str::FromStr>::from_str(&c).unwrap()).await.unwrap();
    // let buf = hyper::body::to_bytes(res).await.unwrap();
    // let  keys= serde_json::from_slice::<Keys>(&buf).unwrap();


    let keys: Keys = surf::get(c.clone()).recv_json().await.unwrap();
    // log::info!("取到了keycloak certs:{:?}", &keys);




    // let body=reqwest::get(c.clone()).await.unwrap().text().await.unwrap();
    // log::info!("{}开始访问body:{:?}",c.clone(),body);

    // let keys: Keys = reqwest::get(c.clone()).await.unwrap().json().await.unwrap();
    log::info!("取到了keycloak certs:{:?}", &keys);


    return keys;
}

pub async fn get_temp_access_token(
    keycloak_auth_server_url: &str,
    keycloak_realm: &str,
    keycloak_resource: &str,
    keycloak_credentials_secret: &str,
) -> String {
    let keycloak_url = format!(
        "{}realms/{}/protocol/openid-connect/token",
        keycloak_auth_server_url, keycloak_realm
    );


    let body_str = format!(
        "client_id={}&client_secret={}&grant_type=client_credentials",
        keycloak_resource, keycloak_credentials_secret
    );
    debug!(
        "开始取临时access_token,参数如下：{},{}",
        &keycloak_url, &body_str
    );

    let access_token: KeyCloakAccessToken = surf::post(keycloak_url)
        .body_string(body_str)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .recv_json()
        .await
        .unwrap();


    // let body=[("client_id",keycloak_resource),("client_secret",keycloak_credentials_secret),("grant_type","client_credentials")];
    // let response = reqwest::Client::new()
    //     .post(keycloak_url)
    //     .form(&body)
    //     .send()
    //     .await
    //     .expect("send");
    //
    // let access_token: KeyCloakAccessToken = response.json().await.unwrap();

    debug!("取到了临时access_token:{:?}", &access_token);
    return access_token.access_token.unwrap();
}
