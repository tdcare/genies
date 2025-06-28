/*
 * @Author: tzw
 * @Date: 2021-10-31 03:05:39
 * @LastEditors: tzw
 * @LastEditTime: 2021-11-29 22:23:13
 */
#[allow(unused_variables)] //允许未使用的变量
#[allow(dead_code)] //允许未使用的代码
#[allow(unused_must_use)]

#[allow(non_snake_case)]

// #[macro_use]
extern crate rbatis;
extern crate rbdc;




pub use genies_core::*;
pub use genies_derive::*;
pub use genies_config::*;
pub use genies_context::*;
pub use genies_cache::*;
pub use genies_dapr::*;
pub use genies_ddd::*;
pub use genies_k8s::*;





/// 获取 rbatis 连接
#[macro_export]
macro_rules! pool {
    () => {
        &mut ddd_dapr::config::CONTEXT.rbatis.clone()
    };
}
/// 获取带守卫功能的事务连接
#[macro_export]
macro_rules! tx_defer {
    () => {
        pool!()
            .acquire_begin()
            .await
            .unwrap()
            .defer_async(|mut tx| async move {
                if !tx.done {
                    tx.rollback().await;
                    log::warn!("tx 没有手动commit 自动执行 rollback，请检查代码！");
                } else {
                    log::debug!("tx 已经 commit 成功");
                }
            })
    };
    // 传入 rbatis 连接
    ($rb:ident) => {
        $rb.acquire_begin()
            .await
            .unwrap()
            .defer_async(|mut tx| async move {
                if !tx.done {
                    tx.rollback().await;
                    log::warn!("tx 没有手动commit 自动执行 rollback，请检查代码！");
                } else {
                    log::debug!("tx 已经 commit 成功");
                }
            })
    };
}
/// 从一个对象中，按字段创建出另一个对象(src 为源对象的借用，dest 为目标对象的类型)
/// 会根据src 对象的字段，新建一个类型为 dest 的新对象
#[macro_export]
macro_rules! copy {
    ($src:expr,$dest:ty) => {
        serde_json::from_slice::<$dest>(&serde_json::to_vec($src).unwrap()).unwrap()
    };
}
/// 用来设置fiegnhttp的 gateway 参数为 服务路由前缀
#[macro_export]
macro_rules! config_gateway {
    ($servlet_path:expr) => {
        once_cell::sync::Lazy::new(|| {
            let service_name = $servlet_path.to_string();
            let mut gateway = ddd_dapr::config::CONTEXT
                .config
                .gateway
                .clone()
                .unwrap_or_default();
            let dapr_url = format!(
                "http://localhost:3500/v1.0/invoke{}-service/method",
                service_name.clone()
            );
            let gateway_url = format!("{}{}", gateway, service_name);

            if gateway.contains("http://") || gateway.contains("https://") {
                gateway.clear();
                gateway.push_str(&gateway_url);
            } else {
                gateway.clear();
                gateway.push_str(&dapr_url);
            }
            gateway
        })
    };
}


#[cfg(test)]
mod tests {

    #[test]
    fn copy_string() {
        let src1 = "测试字符串".to_string();
        let dest = copy!(&src1, String);
        println!("{}", dest);
        assert_eq!(dest, src1);
    }

    #[test]
    fn copy_obj() {
        use serde::{Deserialize, Serialize};
        #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
        struct A {
            name: String,
        }
        #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
        struct B {
            age: u16,
        }
        #[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
        struct C {
            #[serde(flatten)]
            b: Option<B>,
            name: Option<String>,
        }
        let a = A {
            name: "A的名字".to_string(),
        };
        let b = B { age: 10 };
        let c = C {
            b: Option::from(b.clone()),
            name: Option::from("C的名字".to_string()),
        };
        let dest = copy!(&a, C);
        println!("{:?}", dest);
        assert_eq!(
            dest,
            C {
                b: None,
                name: Option::from("A的名字".to_string())
            }
        );

        let dest = copy!(&c, B);
        println!("{:?}", dest);
        assert_eq!(dest, B { age: 10 });

        let mut dest = copy!(&c, A);
        println!("{:?}", dest);
        assert_eq!(
            dest,
            A {
                name: "C的名字".to_string()
            }
        );
        dest.name = "修改的名字".to_string();
        println!("{:?}", dest);
        assert_eq!(
            dest,
            A {
                name: "修改的名字".to_string()
            }
        );
    }
}
