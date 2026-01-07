use async_trait::async_trait;
use boxy_cache::Cache;
use boxy_core::{
    manager::PackageManager,
    package::{Capability, Package},
};
use boxy_core::retry::retry_with_backoff;
use boxy_core::{DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY};
use boxy_error::{BoxyError, Result};
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

pub struct UvManager {
    cache: Arc<Cache>,
    _global: bool,
}

impl UvManager {
    pub fn new(cache: Arc<Cache>, global: bool) -> Self {
        Self {
            cache,
            _global: global,
        }
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        debug!("执行 uv 命令: {}", args.join(" "));

        retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(COMMAND_TIMEOUT, Command::new("uv").args(args).output())
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
                .map_err(|_| BoxyError::CommandFailed {
                    manager: "uv".to_string(),
                    command: args.join(" "),
                    exit_code: -1,
                })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(BoxyError::CommandFailed {
                    manager: "uv".to_string(),
                    command: args.join(" "),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            }
        })
        .await
    }
}

#[async_trait]
impl PackageManager for UvManager {
    fn name(&self) -> &str {
        "uv"
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("uv").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get("uv").await? {
            debug!("使用缓存的 uv 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["pip", "list"]).await?;

        let packages: Vec<Package> = output
            .lines()
            .skip(2)
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                Some(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    manager: "uv".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: None,
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect();

        self.cache.set("uv", &packages).await?;
        debug!("uv 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = self.exec(&["pip", "search", query]).await?;

        let packages: Vec<Package> = output
            .lines()
            .skip(2)
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                let name = parts[0].to_string();
                let version = parts
                    .get(1)
                    .map(|s| s.trim_matches(&['(', ')'] as &[char]).to_string())
                    .unwrap_or_default();

                Some(Package {
                    name,
                    version,
                    manager: "uv".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: None,
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect();

        Ok(packages)
    }

    async fn get_info(&self, name: &str) -> Result<Package> {
        let output = self.exec(&["pip", "show", name]).await?;

        let mut version = String::new();
        let mut description = None;
        let mut homepage = None;
        let mut license = None;

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("Version:") {
                version = line.replace("Version:", "").trim().to_string();
            } else if line.starts_with("Summary:") {
                description = Some(line.replace("Summary:", "").trim().to_string());
            } else if line.starts_with("Home-page:") {
                homepage = Some(line.replace("Home-page:", "").trim().to_string());
            } else if line.starts_with("License:") {
                license = Some(line.replace("License:", "").trim().to_string());
            }
        }

        Ok(Package {
            name: name.to_string(),
            version,
            manager: "uv".to_string(),
            description,
            homepage,
            license,
            installed_path: None,
            size: None,
            outdated: false,
            latest_version: None,
        })
    }

    async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        let mut args: Vec<String> = vec!["pip".to_string(), "install".to_string()];
        let target = match version {
            Some(v) => format!("{}=={}", name, v),
            None => name.to_string(),
        };
        args.push(target);

        info!("uv pip install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("uv").await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("uv pip install --upgrade {}", name);
        self.exec(&["pip", "install", "--upgrade", name]).await?;
        self.cache.invalidate("uv").await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["pip".to_string(), "uninstall".to_string()];
        if force {
            args.push("--yes".to_string());
        }
        args.push(name.to_string());

        warn!("uv pip uninstall {} (force: {})", name, force);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("uv").await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        let output = self.exec(&["pip", "list", "--outdated"]).await?;

        let outdated: Vec<Package> = output
            .lines()
            .skip(2)
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 3 {
                    return None;
                }

                let name = parts[0].to_string();
                let current_version = parts[1].to_string();
                let latest_version = parts[2].trim_matches(&['(', ')'] as &[char]).to_string();

                Some(Package {
                    name,
                    version: current_version,
                    manager: "uv".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: None,
                    size: None,
                    outdated: true,
                    latest_version: Some(latest_version),
                })
            })
            .collect();

        Ok(outdated)
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
    fn test_uv_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = UvManager::new(cache.clone(), false);
        assert_eq!(manager.name(), "uv");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = UvManager::new(cache, false);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
