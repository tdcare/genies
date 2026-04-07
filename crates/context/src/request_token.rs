use tokio::task_local;

task_local! {
    /// 存储当前请求的用户 Authorization header（如 "Bearer xxx"）
    /// 在 salvo_auth 中间件中设置，供 #[remote] 宏和手写远程调用使用
    pub static REQUEST_TOKEN: String;
}

/// 尝试获取当前请求的用户 token
/// 返回 Some(token) 如果在 salvo_auth 中间件设置的 scope 内
/// 返回 None 如果不在请求上下文中（如定时任务、初始化等）
pub fn get_request_token() -> Option<String> {
    REQUEST_TOKEN.try_with(|t| t.clone()).ok()
}
