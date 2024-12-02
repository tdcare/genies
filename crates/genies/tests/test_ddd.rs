// #![feature(raw)]


#[cfg(test)]
pub mod ddd_tests {
    use ddd_dapr::copy;
    use ddd_dapr::event::DomainEvent;
    use ddd_dapr::pool;
    use ddd_dapr::DomainEventPublisher::*;
    use ddd_dapr_derive::Aggregate;
    use ddd_dapr_derive::DomainEvent;
    use serde::*;

    #[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
    #[event_type_version("V1")]
    #[event_source("me.tdcarefor.tdnis.user.domain.aggregate.TestAggregate")]
    #[event_type("me.tdcarefor.tdnis.user.domain.event.TestDomainEvent")]
    pub struct TestDomainEvent {
        // @Id
        // @Column
        // private String id;
        pub id: Option<String>,

        // @ApiModelProperty(value ="用户id")
        // private String userId;
        pub userId: Option<String>,
        // @ApiModelProperty(value ="用户真实姓名")
        // private String userRealName;
        pub userRealName: Option<String>,
        pub departmentId: Option<String>,
    }
    #[derive(DomainEvent, Debug, Serialize, Deserialize, Default, Clone, PartialEq)]
    #[event_type_version("V2")]
    #[event_source("me.tdcarefor.tdnis.user.domain.aggregate.TestAggregate")]
    #[event_type("me.tdcarefor.tdnis.user.domain.event.TestDomainEvent2")]
    pub struct TestDomainEvent2 {
        // @Id
        // @Column
        // private String id;
        pub id: Option<String>,

        // @ApiModelProperty(value ="用户id")
        // private String userId;
        pub userId: Option<String>,

        // @ApiModelProperty(value ="用户真实姓名")
        // private String userRealName;
        pub userRealName: Option<String>,
    }
    #[derive(Debug, serde::Serialize, serde::Deserialize, Clone, Default, PartialEq, Aggregate)]
    #[aggregate_type("me.tdcarefor.tdnis.user.domain.aggregate.TestAggregate")]
    #[id_field(id)]
    pub struct TestAggregate {
        pub id: Option<String>,
        pub departmentId: Option<String>,
    }
    impl TestAggregate {
        pub async fn get_domain_event(
            root: &TestAggregate,
        ) -> (TestAggregate, Box<dyn DomainEvent>) {
            let event = copy!(root, TestDomainEvent);
            (root.clone(), Box::new(event))
        }
    }
    #[tokio::test]
    async fn publisher_root_domain_event_test() {
        let (root, event) = TestAggregate::get_domain_event(&TestAggregate {
            id: Some("123".to_string()),
            departmentId: Some("ddd-0001".to_string()),
        })
        .await;
        publish(pool!(), &root, event).await;
    }

    #[tokio::test]
    async fn publisher_generic_domain_event_test() {
        let test_event = Box::new(TestDomainEvent {
            id: Some("test_id".to_string()),
            userId: Some("tzw".to_string()),
            userRealName: Some("谭".to_string()),
            departmentId: None,
        });
        let m = buildGenericMessage(test_event);

        println!("{:?}", m);
        let events = get_event().await;
        for event in events {
            println!("{:?}", event.event_type());
            println!("{}", event.json());
            publishGenericDomainEvent(pool!(), event).await;
        }
    }

    // 这个函数返回 dyn DomainEvent 会报错
    pub async fn get_event() -> Vec<Box<dyn DomainEvent>> {
        let mut domain_events: Vec<Box<dyn DomainEvent>> = vec![];
        domain_events.push(Box::new(TestDomainEvent2 {
            id: Some("id1".to_string()),
            userId: None,
            userRealName: None,
        }));
        domain_events.push(Box::new(TestDomainEvent {
            id: Some("id2".to_string()),
            userId: None,
            userRealName: None,
            departmentId: None,
        }));
        return domain_events;
    }
}
