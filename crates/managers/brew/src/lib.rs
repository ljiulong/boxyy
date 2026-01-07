use async_trait::async_trait;
use boxy_cache::Cache;
use boxy_core::{
    manager::PackageManager,
    package::{Capability, Package},
};
use boxy_core::retry::retry_with_backoff;
use boxy_core::{DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY};
use boxy_error::{BoxyError, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

pub struct BrewManager {
    cache: Arc<Cache>,
}

impl BrewManager {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache }
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        debug!("执行 brew 命令: {}", args.join(" "));

        retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(COMMAND_TIMEOUT, Command::new("brew").args(args).output())
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
                .map_err(|_| BoxyError::CommandFailed {
                    manager: "brew".to_string(),
                    command: args.join(" "),
                    exit_code: -1,
                })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(BoxyError::CommandFailed {
                    manager: "brew".to_string(),
                    command: args.join(" "),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            }
        })
        .await
    }

    fn parse_list_output_with_versions(&self, output: &str) -> Vec<Package> {
        output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                let mut parts = line.split_whitespace();
                let name = parts.next()?.to_string();
                let version = parts.next().unwrap_or_default().to_string();

                Some(Package {
                    name,
                    version,
                    manager: "brew".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: None,
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect()
    }

    fn parse_search_output(&self, output: &str) -> Vec<Package> {
        output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with("==>") {
                    return None;
                }

                // brew search 输出格式: package_name
                Some(Package {
                    name: line.to_string(),
                    version: String::new(),
                    manager: "brew".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: None,
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect()
    }

    fn parse_outdated_output(&self, output: &str) -> Vec<Package> {
        output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // brew outdated 输出格式: package_name (current_version) < latest_version
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                let name = parts[0].to_string();
                let current_version = parts
                    .get(1)
                    .map(|s| s.trim_matches(&['(', ')'] as &[char]).to_string())
                    .unwrap_or_default();
                let latest_version = parts.get(3).map(|s| s.to_string()).unwrap_or_default();

                Some(Package {
                    name,
                    version: current_version,
                    manager: "brew".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: None,
                    size: None,
                    outdated: true,
                    latest_version: Some(latest_version),
                })
            })
            .collect()
    }

    fn parse_license(value: &Value) -> Option<String> {
        if let Some(license) = value.as_str() {
            return Some(license.to_string());
        }

        if let Some(list) = value.as_array() {
            let licenses = list
                .iter()
                .filter_map(|item| item.as_str())
                .collect::<Vec<_>>();
            if !licenses.is_empty() {
                return Some(licenses.join(", "));
            }
        }

        None
    }

    fn parse_size_from_installed(value: &Value) -> Option<u64> {
        value
            .get("installed")
            .and_then(|installed| installed.as_array())
            .and_then(|installed| installed.first())
            .and_then(|item| item.get("installed_size"))
            .and_then(|size| size.as_u64())
            .map(|size| size.saturating_mul(1024))
    }

    async fn fetch_installed_sizes(&self) -> Result<HashMap<String, u64>> {
        let output = self.exec(&["info", "--json=v2", "--installed"]).await?;
        let data: Value =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 brew info 输出失败: {}", e),
            })?;

        let mut sizes = HashMap::new();
        if let Some(formulae) = data.get("formulae").and_then(|list| list.as_array()) {
            for formula in formulae {
                let name = match formula.get("name").and_then(|value| value.as_str()) {
                    Some(value) => value,
                    None => continue,
                };
                if let Some(size) = Self::parse_size_from_installed(formula) {
                    sizes.insert(name.to_string(), size);
                }
            }
        }

        if let Some(casks) = data.get("casks").and_then(|list| list.as_array()) {
            for cask in casks {
                let name = cask
                    .get("token")
                    .and_then(|value| value.as_str())
                    .or_else(|| cask.get("name").and_then(|value| value.as_str()));
                let name = match name {
                    Some(value) => value,
                    None => continue,
                };
                if let Some(size) = cask.get("installed_size").and_then(|value| value.as_u64()) {
                    sizes.insert(name.to_string(), size.saturating_mul(1024));
                }
            }
        }

        Ok(sizes)
    }

    fn parse_json_info(&self, output: &str, name: &str) -> Result<Package> {
        let data: Value =
            serde_json::from_str(output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 brew info 输出失败: {}", e),
            })?;

        if let Some(formula) = data
            .get("formulae")
            .and_then(|list| list.as_array())
            .and_then(|list| list.first())
        {
            let version = formula
                .get("versions")
                .and_then(|value| value.get("stable"))
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let description = formula
                .get("desc")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let homepage = formula
                .get("homepage")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let license = formula
                .get("license")
                .and_then(Self::parse_license);
            let size = Self::parse_size_from_installed(formula);

            return Ok(Package {
                name: formula
                    .get("name")
                    .and_then(|value| value.as_str())
                    .unwrap_or(name)
                    .to_string(),
                version,
                manager: "brew".to_string(),
                description,
                homepage,
                license,
                installed_path: None,
                size,
                outdated: false,
                latest_version: None,
            });
        }

        if let Some(cask) = data
            .get("casks")
            .and_then(|list| list.as_array())
            .and_then(|list| list.first())
        {
            let version = cask
                .get("version")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string())
                .or_else(|| {
                    cask.get("version").and_then(|value| {
                        value.as_array().map(|items| {
                            items
                                .iter()
                                .filter_map(|item| item.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                    })
                })
                .unwrap_or_default();
            let description = cask
                .get("desc")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let homepage = cask
                .get("homepage")
                .and_then(|value| value.as_str())
                .map(|value| value.to_string());
            let size = cask.get("installed_size").and_then(|value| value.as_u64());

            return Ok(Package {
                name: cask
                    .get("token")
                    .and_then(|value| value.as_str())
                    .unwrap_or(name)
                    .to_string(),
                version,
                manager: "brew".to_string(),
                description,
                homepage,
                license: None,
                installed_path: None,
                size: size.map(|value| value.saturating_mul(1024)),
                outdated: false,
                latest_version: None,
            });
        }

        Err(BoxyError::JsonError {
            message: "解析 brew info 输出失败".to_string(),
        })
    }
}

#[async_trait]
impl PackageManager for BrewManager {
    fn name(&self) -> &str {
        "brew"
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("brew").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get("brew").await? {
            debug!("使用缓存的 brew 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["list", "--versions"]).await?;
        let mut packages = self.parse_list_output_with_versions(&output);

        match self.exec(&["list", "--cask", "--versions"]).await {
            Ok(cask_output) => {
                let mut cask_packages = self.parse_list_output_with_versions(&cask_output);
                packages.append(&mut cask_packages);
            }
            Err(err) => {
                warn!("brew 获取 cask 列表失败: {}", err);
            }
        }

        match self.fetch_installed_sizes().await {
            Ok(size_map) => {
                for pkg in packages.iter_mut() {
                    if let Some(size) = size_map.get(&pkg.name) {
                        pkg.size = Some(*size);
                    }
                }
            }
            Err(err) => {
                warn!("brew 获取包大小失败: {}", err);
            }
        }

        self.cache.set("brew", &packages).await?;
        debug!("brew 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = self.exec(&["search", query]).await?;
        Ok(self.parse_search_output(&output))
    }

    async fn get_info(&self, name: &str) -> Result<Package> {
        let output = self.exec(&["info", "--json=v2", name]).await?;
        if let Ok(pkg) = self.parse_json_info(&output, name) {
            return Ok(pkg);
        }
        let output = self.exec(&["info", "--json=v2", "--cask", name]).await?;
        self.parse_json_info(&output, name)
    }

    async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        let mut args = vec!["install".to_string()];
        if let Some(v) = version {
            args.push(format!("{}@{}", name, v));
        } else {
            args.push(name.to_string());
        }

        info!("brew install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        if self.exec(&args_refs).await.is_err() {
            self.exec(&["install", "--cask", name]).await?;
        }
        self.cache.invalidate("brew").await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("brew upgrade {}", name);
        if self.exec(&["upgrade", name]).await.is_err() {
            self.exec(&["upgrade", "--cask", name]).await?;
        }
        self.cache.invalidate("brew").await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args = vec!["uninstall".to_string()];
        if force {
            args.push("--force".to_string());
        }
        args.push(name.to_string());

        warn!("brew uninstall {} (force: {})", name, force);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        if self.exec(&args_refs).await.is_err() {
            let mut cask_args = vec!["uninstall".to_string(), "--cask".to_string()];
            if force {
                cask_args.push("--force".to_string());
            }
            cask_args.push(name.to_string());
            let cask_refs: Vec<&str> = cask_args.iter().map(|s| s.as_str()).collect();
            self.exec(&cask_refs).await?;
        }
        self.cache.invalidate("brew").await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        let output = self.exec(&["outdated"]).await?;
        Ok(self.parse_outdated_output(&output))
    }

    fn capabilities(&self) -> &[Capability] {
        use Capability::*;

        &[ListInstalled, SearchRemote, VersionSelection]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brew_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = BrewManager::new(cache);
        assert_eq!(manager.name(), "brew");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = BrewManager::new(cache);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
