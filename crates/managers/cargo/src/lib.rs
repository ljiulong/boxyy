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

pub struct CargoManager {
    cache: Arc<Cache>,
    _global: bool,
}

impl CargoManager {
    pub fn new(cache: Arc<Cache>, global: bool) -> Self {
        Self {
            cache,
            _global: global,
        }
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        debug!("执行 cargo 命令: {}", args.join(" "));

        retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(COMMAND_TIMEOUT, Command::new("cargo").args(args).output())
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
                .map_err(|_| BoxyError::CommandFailed {
                    manager: "cargo".to_string(),
                    command: args.join(" "),
                    exit_code: -1,
                })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(BoxyError::CommandFailed {
                    manager: "cargo".to_string(),
                    command: args.join(" "),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            }
        })
        .await
    }
}

#[async_trait]
impl PackageManager for CargoManager {
    fn name(&self) -> &str {
        "cargo"
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("cargo").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get("cargo").await? {
            debug!("使用缓存的 cargo 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["install", "--list"]).await?;

        let packages: Vec<Package> = output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // cargo install --list 输出格式: package_name version:
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                let name = parts[0].to_string();
                let version = parts
                    .get(1)
                    .map(|s| s.trim_matches(':').to_string())
                    .unwrap_or_default();

                Some(Package {
                    name,
                    version,
                    manager: "cargo".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: Some("~/.cargo/bin".to_string()),
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect();

        self.cache.set("cargo", &packages).await?;
        debug!("cargo 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = self.exec(&["search", query]).await?;

        let packages: Vec<Package> = output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with("=") {
                    return None;
                }

                // cargo search 输出格式: package_name = "version" # description
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() < 2 {
                    return None;
                }

                let name = parts[0].trim().to_string();
                let version_part = parts[1].split('#').next().unwrap_or("").trim();
                let version = version_part
                    .trim_matches(&['"', ' '] as &[char])
                    .to_string();

                let description = if parts.len() > 1 {
                    parts[1].split('#').nth(1).map(|s| s.trim().to_string())
                } else {
                    None
                };

                Some(Package {
                    name,
                    version,
                    manager: "cargo".to_string(),
                    description,
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
        let output = self.exec(&["search", name, "--limit", "1"]).await?;

        let mut version = String::new();
        let mut description = None;

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with(name) && line.contains('=') {
                let parts: Vec<&str> = line.split('=').collect();
                if parts.len() >= 2 {
                    version = parts[1]
                        .split('#')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .trim_matches(&['"', ' '] as &[char])
                        .to_string();
                    if parts.len() > 1 {
                        description = parts[1].split('#').nth(1).map(|s| s.trim().to_string());
                    }
                }
                break;
            }
        }

        Ok(Package {
            name: name.to_string(),
            version,
            manager: "cargo".to_string(),
            description,
            homepage: None,
            license: None,
            installed_path: Some("~/.cargo/bin".to_string()),
            size: None,
            outdated: false,
            latest_version: None,
        })
    }

    async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        let mut args: Vec<String> = vec!["install".to_string()];
        if let Some(v) = version {
            args.push("--version".to_string());
            args.push(v.to_string());
        }
        args.push(name.to_string());

        info!("cargo install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("cargo").await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("cargo install --force {}", name);
        self.exec(&["install", "--force", name]).await?;
        self.cache.invalidate("cargo").await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        warn!("cargo uninstall {} (force: {})", name, force);
        self.exec(&["uninstall", name]).await?;
        self.cache.invalidate("cargo").await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        // cargo 没有直接的 outdated 命令，需要手动检查
        let installed = self.list_installed().await?;
        let mut outdated = Vec::new();

        for pkg in installed {
            match self.get_info(&pkg.name).await {
                Ok(info) => {
                    if info.latest_version.is_some()
                        && info.latest_version.as_ref() != Some(&pkg.version)
                    {
                        outdated.push(Package {
                            name: pkg.name,
                            version: pkg.version,
                            manager: "cargo".to_string(),
                            description: None,
                            homepage: None,
                            license: None,
                            installed_path: pkg.installed_path,
                            size: None,
                            outdated: true,
                            latest_version: info.latest_version,
                        });
                    }
                }
                Err(_) => continue,
            }
        }

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
    fn test_cargo_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = CargoManager::new(cache.clone(), true);
        assert_eq!(manager.name(), "cargo");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = CargoManager::new(cache, true);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
