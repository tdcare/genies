/*
 * @Author: tzw
 * @Date: 2021-10-31 03:05:39
 * @LastEditors: tzw
 * @LastEditTime: 2021-11-29 22:23:13
 */
pub mod error;
pub mod jwt;


/// 自定义 Result 类型，接受两个泛型参数：T 为成功时的返回类型，E 为错误类型
pub type Result<T> = std::result::Result<T, crate::error::Error>;



use crate::error::Error;
// use actix_http::Response;
use salvo::prelude::*;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

// pub mod jwt;

pub const CODE_SUCCESS: &str = "SUCCESS";
pub const CODE_FAIL: &str = "FAIL";

pub const CODE_SUCCESS_I32: i32 = 1;
pub const CODE_FAIL_I32: i32 = 0;

/// http接口返回模型结构，提供基础的 code，msg，data 等json数据结构
#[derive(Debug, Serialize, Deserialize, Clone,Default)]
pub struct RespVO<T> {
    pub code: Option<String>,
    pub msg: Option<String>,
    pub data: Option<T>,
}

#[async_trait]
impl<T> Writer for RespVO<T>
    where
        T: Serialize + DeserializeOwned + Clone+Send,
{
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.headers_mut().insert("Content-Type", "text/json;charset=UTF-8".parse().unwrap());
        match self.code {
            Some(code) if code.eq("dapr") =>{
                res.render(serde_json::to_string(&self.data.unwrap()).unwrap());
            },
            _=>{
                res.render(self.to_string());
            }
        }
    }
}
impl<T> RespVO<T>
where
    T: Serialize + DeserializeOwned + Clone+Send,
{
    pub fn from_result(arg: &Result<T>) -> Self {
        if arg.is_ok() {
            Self {
                code: Some(CODE_SUCCESS.to_string()),
                msg: None,
                data: arg.clone().ok(),
            }
        } else {
            Self {
                code: Some(CODE_FAIL.to_string()),
                msg: Some(arg.clone().err().unwrap().to_string()),
                data: None,
            }
        }
    }

    pub fn from(arg: &T) -> Self {
        Self {
            code: Some(CODE_SUCCESS.to_string()),
            msg: None,
            data: Some(arg.clone()),
        }
    }

    pub fn from_error(code: &str, arg: &Error) -> Self {
        let mut code_str = code.to_string();
        if code_str.is_empty() {
            code_str = CODE_FAIL.to_string();
        }
        Self {
            code: Some(code_str),
            msg: Some(arg.to_string()),
            data: None,
        }
    }

    pub fn from_error_info(code: &str, info: &str) -> Self {
        let mut code_str = code.to_string();
        if code_str.is_empty() {
            code_str = CODE_FAIL.to_string();
        }
        Self {
            code: Some(code_str),
            msg: Some(info.to_string()),
            data: None,
        }
    }


    pub fn resp_json(&self) -> Self {
        Self {
            code: self.code.clone(),
            msg: self.msg.clone(),
            data: self.data.clone(),
        }
    }



    /// Dapr pubsub  所需要的返回值
    pub fn to_dapr_pubsub() -> Self {
        Self{
            code: None,
            msg: None,
            data: None
        }
    }
    // /// 标记 Dapr pubsub 消息重试
    pub fn is_retry(&mut self) ->RespVO::<serde_json::Value>  {
        RespVO::<serde_json::Value> {
            code: Some("dapr".to_string()),
            msg: None,
            data: Some(json!({"status": "RETRY"})),
        }
    }
    /// 标记 Dapr pubsub 消息消费成功
    pub fn is_success(&mut self)-> RespVO::<serde_json::Value> {
        RespVO::<serde_json::Value> {
            code: Some("dapr".to_string()),
            msg: None,
            data: Some(json!({"status": "SUCCESS"})),
        }

    }

}

impl<T> ToString for RespVO<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

/// 兼容Java返回值
#[derive(Debug, Serialize, Deserialize, Clone,Default)]
pub struct ResultDTO<T> {
    pub status: Option<i32>,
    pub message: Option<String>,
    pub data: Option<T>,
}
#[async_trait]
impl<T> Writer for ResultDTO<T>
    where
        T: Serialize + DeserializeOwned + Clone+Send,
{
    async fn write(mut self, _req: &mut Request, _depot: &mut Depot, res: &mut Response) {
        res.headers_mut().insert("Content-Type", "text/json;charset=UTF-8".parse().unwrap());
        match self.status {
            _=>{
                res.render(self.to_string());
            }
        }
    }
}

impl<T> ResultDTO<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    pub fn from_result(arg: &Result<T>) -> Self {
        if arg.is_ok() {
            Self {
                status: Some(CODE_SUCCESS_I32),
                message: None,
                data: arg.clone().ok(),
            }
        } else {
            Self {
                status: Some(CODE_FAIL_I32),
                message: Some(arg.clone().err().unwrap().to_string()),
                data: None,
            }
        }
    }

    pub fn from(arg: &T) -> Self {
        Self {
            status: Some(CODE_SUCCESS_I32),
            message: None,
            data: Some(arg.clone()),
        }
    }
    pub fn from_message(data: &T, message: &str) -> Self {
        Self {
            status: Some(CODE_SUCCESS_I32),
            message: Some(message.to_string()),
            data: Some(data.clone()),
        }
    }
    pub fn from_code_message(code: i32, message: &str, data: &T) -> Self {
        Self {
            status: Some(code),
            message: Some(message.to_string()),
            data: Some(data.clone()),
        }
    }

    pub fn from_error(code: i32, message: &Error) -> Self {
        // let mut code_str = code.to_string();
        // if code_str.is_empty() {
        //     code_str = CODE_FAIL.to_string();
        // }
        Self {
            status: Some(code),
            message: Some(message.to_string()),
            data: None,
        }
    }

    pub fn from_error_info(code: i32, message: &str) -> Self {
        // let mut code_str = code.to_string();
        // if code_str.is_empty() {
        //     code_str = CODE_FAIL.to_string();
        // }
        Self {
            status: Some(code),
            message: Some(message.to_string()),
            data: None,
        }
    }

    pub fn resp_json(&self) -> Self {
        Self {
            status: self.status.clone(),
            data: self.data.clone(),
            message: self.message.clone()
        }
    }
}

impl<T> ToString for ResultDTO<T>
where
    T: Serialize + DeserializeOwned + Clone,
{
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

