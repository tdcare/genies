/*
 * @Author: tzw
 * @Date: 2021-11-29 22:12:35
 * @LastEditors: tzw
 * @LastEditTime: 2022-01-17 01:49:34
 */


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
