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

pub struct PipxManager {
    cache: Arc<Cache>,
}

impl PipxManager {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache }
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        debug!("执行 pipx 命令: {}", args.join(" "));

        retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(COMMAND_TIMEOUT, Command::new("pipx").args(args).output())
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
                .map_err(|_| BoxyError::CommandFailed {
                    manager: "pipx".to_string(),
                    command: args.join(" "),
                    exit_code: -1,
                })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(BoxyError::CommandFailed {
                    manager: "pipx".to_string(),
                    command: args.join(" "),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            }
        })
        .await
    }

    fn parse_list_output(&self, output: &str) -> Vec<Package> {
        output
            .lines()
            .skip(1) // 跳过标题行
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // pipx list 输出格式: package_name version
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                let name = parts[0].to_string();
                let version = parts.get(1).unwrap_or(&"").to_string();

                Some(Package {
                    name,
                    version,
                    manager: "pipx".to_string(),
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
}

#[async_trait]
impl PackageManager for PipxManager {
    fn name(&self) -> &str {
        "pipx"
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("pipx").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get("pipx").await? {
            debug!("使用缓存的 pipx 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["list"]).await?;
        let packages = self.parse_list_output(&output);

        self.cache.set("pipx", &packages).await?;
        debug!("pipx 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // pipx 不支持搜索，使用 pip search
        let output = retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(
                COMMAND_TIMEOUT,
                Command::new("pip").args(["search", query]).output(),
            )
            .await
            .map_err(|_| BoxyError::CommandTimeout)?
            .map_err(|_| BoxyError::CommandFailed {
                manager: "pipx".to_string(),
                command: format!("pip search {}", query),
                exit_code: -1,
            })?;
            Ok(output)
        })
        .await?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let packages: Vec<Package> = output_str
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
                    manager: "pipx".to_string(),
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
        // pipx 没有直接的 info 命令，使用 pip show
        let output = retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(
                COMMAND_TIMEOUT,
                Command::new("pip").args(["show", name]).output(),
            )
            .await
            .map_err(|_| BoxyError::CommandTimeout)?
            .map_err(|_| BoxyError::CommandFailed {
                manager: "pipx".to_string(),
                command: format!("pip show {}", name),
                exit_code: -1,
            })?;
            Ok(output)
        })
        .await?;

        if !output.status.success() {
            return Err(BoxyError::CommandFailed {
                manager: "pipx".to_string(),
                command: format!("pip show {}", name),
                exit_code: output.status.code().unwrap_or(-1),
            });
        }

        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut version = String::new();
        let mut description = None;

        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("Version:") {
                version = line.replace("Version:", "").trim().to_string();
            } else if line.starts_with("Summary:") {
                description = Some(line.replace("Summary:", "").trim().to_string());
            }
        }

        Ok(Package {
            name: name.to_string(),
            version,
            manager: "pipx".to_string(),
            description,
            homepage: None,
            license: None,
            installed_path: None,
            size: None,
            outdated: false,
            latest_version: None,
        })
    }

    async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        let mut args: Vec<String> = vec!["install".to_string()];
        let target = match version {
            Some(v) => format!("{}=={}", name, v),
            None => name.to_string(),
        };
        args.push(target);

        info!("pipx install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("pipx").await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("pipx upgrade {}", name);
        self.exec(&["upgrade", name]).await?;
        self.cache.invalidate("pipx").await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["uninstall".to_string()];
        if force {
            args.push("--force".to_string());
        }
        args.push(name.to_string());

        warn!("pipx uninstall {} (force: {})", name, force);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("pipx").await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        let output = self.exec(&["list"]).await?;
        let installed = self.parse_list_output(&output);
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
                            manager: "pipx".to_string(),
                            description: None,
                            homepage: None,
                            license: None,
                            installed_path: None,
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

        &[ListInstalled, VersionSelection]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipx_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PipxManager::new(cache);
        assert_eq!(manager.name(), "pipx");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PipxManager::new(cache);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
