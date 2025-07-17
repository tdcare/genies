use genies_derive::topic;
use crate::DeviceUseEvent::DeviceUseEvent;

/// 处理设备使用开始事件，发送ADT指令
#[topic(
    name = "me.tdcarefor.tdnis.device.domain.DeptDeviceEntity",
    pubsub = "messagebus"
)]
pub async fn onDeviceUseEvent(tx: &mut dyn Executor, event: DeviceUseEvent) -> anyhow::Result<u64> {
    // return UseDeviceService::onDeviceUseEvent(tx, event).await;
    Ok(0)
}
/// 处理设备使用开始事件，发送ADT指令1
#[topic(
    name = "me.tdcarefor.tdnis.device.domain.DeptDeviceEntity",
    pubsub = "messagebus"
)]
pub async fn onDeviceUseEvent1(tx: &mut dyn Executor, event: DeviceUseEvent) -> anyhow::Result<u64> {
    // return UseDeviceService::onDeviceUseEvent(tx, event).await;
    Ok(0)
}
