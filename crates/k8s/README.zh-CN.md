# K8s 模块

`k8s` 模块与 Kubernetes 相关的功能模块。

## 使用说明
```rust
use k8s::K8sClient;

let client = K8sClient::new();
let pods = client.list_pods("default").unwrap();
println!("Pods: {:?}", pods);
```

## 贡献
欢迎提交 Pull Request 或 Issue 来改进本项目。

## 许可证
本项目采用 MIT 许可证，详情请参阅 `LICENSE` 文件。 