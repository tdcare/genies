use salvo::prelude::*;
use log::debug;

#[handler]
pub async fn dapr_sub(req: &mut Request,_depot: &mut Depot,_res: &mut Response, _ctrl: &mut FlowCtrl) {
    debug!("收到的 headers:{:?}",req.headers());
    let body=req.payload().await.unwrap();
    debug!("收到的 body:{}",std::str::from_utf8(body).unwrap());

    if _depot.contains_key("is_retry"){
        debug!("有消费者处理失败,消息进行重发!");
        _res.render(Text::Json(r#"{"status": "RETRY"}"#));
    }else{
    debug!("所有消费者处理成功，此条消息进行确认");
    _res.render(Text::Json(r#"{"status": "SUCCESS"}"#));
    }
}




