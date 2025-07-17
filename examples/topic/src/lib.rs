use std::sync::Arc;
use salvo::Router;

mod UseDeviceListeners;
mod DeviceUseEvent;

use genies::dapr::dapr_sub::dapr_sub;
use crate::UseDeviceListeners::{onDeviceUseEvent1_hoop, onDeviceUseEvent_hoop};

pub fn event_consumer_config() -> Router {


    let router2=Router::new().push(
        Router::with_path("/daprsub/consumers")
            .hoop(onDeviceUseEvent_hoop)
            .hoop(onDeviceUseEvent1_hoop)
            .post(dapr_sub)
    );
    router2
}



#[cfg(test)]
mod tests{
    use salvo::Router;
    use genies::dapr::dapr_sub::dapr_sub;
    use genies::dapr::topicpoint::Topicpoint;

    #[test]
    fn test_inventory() {
        use crate::UseDeviceListeners::*;
        println!("开始");
        let mut router = Router::new().push(
            Router::with_path("/daprsub/consumers")
                .post(dapr_sub)
        );;
        let mut routers = router.routers_mut();

        for record in genies::context::inventory::iter::<Topicpoint> {
           let hoop=(record.hoop)();
            // routers.push(hoop);

             // println!("topic_point: {:?},{:#?}", (record.subscribe)(),router);
             // router.push((record.hoop)());
        }
        // router.post(dapr_sub);

        println!("topic_point: {:#?}", router);
        println!("hoop: {:#?}", router.hoops().iter().count());

      let router1=Router::new().push(
            Router::with_path("/daprsub/consumers")
                .hoop(onDeviceUseEvent_hoop)
                .hoop(onDeviceUseEvent1_hoop)
                .post(dapr_sub)
        );
        println!("topic_point1: {:#?},{:#?}", router1,router1.hoops().iter().count());

        let topic1=Router::with_path("/daprsub/consumers")
            .hoop(onDeviceUseEvent_hoop);
        let topic2=Router::with_path("/daprsub/consumers")
            .hoop(onDeviceUseEvent1_hoop);
        let router2=Router::new().push(
            Router::with_path("/daprsub/consumers")
                .post(dapr_sub)
        );
        println!("topic_point2: {:#?},{:#?}", router2,router2.hoops().iter().count());
        // assert!(!topic_points.is_empty());
    }
}