use crate::package::{Capability, Package};
use async_trait::async_trait;
use boxy_error::{BoxyError, Result};

#[async_trait]
pub trait PackageManager: Send + Sync {
    fn name(&self) -> &str;

    async fn check_available(&self) -> Result<bool>;

    async fn list_installed(&self) -> Result<Vec<crate::package::Package>>;

    async fn search(&self, query: &str) -> Result<Vec<crate::package::Package>>;

    async fn get_info(&self, name: &str) -> Result<crate::package::Package>;

    async fn install(&self, name: &str, version: Option<&str>) -> Result<()>;

    async fn upgrade(&self, name: &str) -> Result<()>;

    async fn uninstall(&self, name: &str, force: bool) -> Result<()>;

    async fn check_outdated(&self) -> Result<Vec<crate::package::Package>>;

    async fn list_dependencies(&self, _name: &str) -> Result<Vec<Package>> {
        Err(BoxyError::UnsupportedOperation {
            manager: self.name().to_string(),
            operation: "list_dependencies".to_string(),
        })
    }

    fn capabilities(&self) -> &[Capability];

    fn cache_key(&self) -> &str {
        self.name()
    }

    fn supports(&self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }
}
