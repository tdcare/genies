/*
 * @Author: tzw
 * @Date: 2021-11-29 22:12:35
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-17 01:49:34
 */

// pub trait DomainEvent: serde::Serialize {
//     fn event_type_version(&self) -> &'static str;
//     fn event_type(&self) -> &'static str;
//     fn event_source(&self) -> &'static str;
// }

// pub trait DomainEvent: serde::Serialize {
//     fn event_type_version(&self) -> String;
//     fn event_type(&self) -> String;
//     fn event_source(&self) -> String;
// }

// serde::Serialize 这个 trait 需要一个lifttime 泛型参数，因此先去掉这个 trait
pub trait DomainEvent:Send {
    /// 领域事件 版本
    fn event_type_version(&self) -> String;
    /// 领域事件 类型
    fn event_type(&self) -> String;
    /// 领域事件 来源
    fn event_source(&self) -> String;
    /// 领域事件 json 字符串
    fn json(&self) -> String;
}
