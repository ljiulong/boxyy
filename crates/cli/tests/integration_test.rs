use boxy_cache::Cache;
use boxy_cli::managers::{create_manager, MANAGER_NAMES};
use std::sync::Arc;

#[tokio::test]
async fn test_create_all_managers() {
    let cache = Arc::new(Cache::new().unwrap());
    for name in MANAGER_NAMES {
        let manager = create_manager(name, cache.clone(), false);
        assert!(manager.is_some(), "应该能创建 {} 管理器", name);
        if let Some(m) = manager {
            assert_eq!(m.name(), name);
        }
    }
}

#[tokio::test]
async fn test_manager_check_available() {
    let cache = Arc::new(Cache::new().unwrap());
    let manager = create_manager("npm", cache, false);

    if let Some(m) = manager {
        // 这个测试可能会失败如果系统没有安装 npm，这是正常的
        let _available = m.check_available().await;
    }
}

#[tokio::test]
async fn test_manager_capabilities() {
    let cache = Arc::new(Cache::new().unwrap());
    let manager = create_manager("npm", cache, false);

    if let Some(m) = manager {
        let caps = m.capabilities();
        assert!(!caps.is_empty(), "npm 应该至少有一个能力");
    }
}
