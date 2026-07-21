//! ResultDTO 兼容层
//!
//! 对应 Java: me.tdcarefor.util.ResultDTO<T>
//! JSON 格式与 Java 版本完全一致

use serde::{Deserialize, Serialize};

/// 统一返回结果 DTO
///
/// status: 0=失败, 1=成功
/// message: 消息
/// data: 返回数据 (可选)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResultDTO<T: Serialize> {
    pub status: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ResultDTO<T> {
    /// 成功（无数据）
    pub fn success() -> ResultDTO<T> {
        ResultDTO {
            status: 1,
            message: "操作完成".to_string(),
            data: None,
        }
    }

    /// 成功（带数据）
    pub fn success_with_data(data: T) -> ResultDTO<T> {
        ResultDTO {
            status: 1,
            message: "操作完成".to_string(),
            data: Some(data),
        }
    }

    /// 成功（自定义消息和数据）
    pub fn success_with_msg(message: &str, data: T) -> ResultDTO<T> {
        ResultDTO {
            status: 1,
            message: message.to_string(),
            data: Some(data),
        }
    }

    /// 失败（默认消息）
    pub fn failed() -> ResultDTO<T> {
        ResultDTO {
            status: 0,
            message: "操作失败".to_string(),
            data: None,
        }
    }

    /// 失败（自定义消息）
    pub fn failed_with_msg(message: &str) -> ResultDTO<T> {
        ResultDTO {
            status: 0,
            message: message.to_string(),
            data: None,
        }
    }

    /// 失败（自定义状态码和消息）
    pub fn failed_with_code(code: i32, message: &str) -> ResultDTO<T> {
        ResultDTO {
            status: code,
            message: message.to_string(),
            data: None,
        }
    }

    /// 根据 bool 返回成功或失败
    pub fn status(status: bool) -> ResultDTO<T> {
        if status {
            ResultDTO::success()
        } else {
            ResultDTO::failed()
        }
    }

    /// 自定义状态码
    pub fn other(code: i32, message: &str) -> ResultDTO<T> {
        ResultDTO {
            status: code,
            message: message.to_string(),
            data: None,
        }
    }

    /// 自定义状态码（带数据）
    pub fn other_with_data(code: i32, message: &str, data: T) -> ResultDTO<T> {
        ResultDTO {
            status: code,
            message: message.to_string(),
            data: Some(data),
        }
    }

    /// 是否成功
    pub fn is_success(&self) -> bool {
        self.status == 1
    }

    /// 是否成功且数据不为空
    pub fn is_success_and_data_not_null(&self) -> bool {
        self.is_success() && self.data.is_some()
    }
}
