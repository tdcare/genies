use genies_derive::DomainEvent;

use serde::{Deserialize, Serialize};

#[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
#[event_type_version("V2")]
#[event_source("me.tdcarefor.tdnis.device.domain.DeptDeviceEntity")]
#[event_type("me.tdcarefor.tdnis.device.event.DeviceUseEvent")]
pub struct DeviceUseEvent {
    // private Integer id;
    pub id: Option<i64>,
    // @ApiModelProperty(value = "名称")
    // @Column(length = 127)
    // private String name;
    pub name: Option<String>,
    // @ApiModelProperty(value = "类型编号")
    // @Column(length = 36)
    // private String deviceNo;
    pub deviceNo: Option<String>,
    // @ApiModelProperty(value = "类型型号")
    // @Column(length = 36)
    // private String deviceModel;
    pub deviceModel: Option<String>,
    // @ApiModelProperty(value = "类型id")
    // @Column(length = 36)
    // private String typeId;
    pub typeId: Option<String>,
    // @ApiModelProperty(value = "扫描码")
    // @Column(length = 64)
    // private String scanNo;
    pub scanNo: Option<String>,

}

