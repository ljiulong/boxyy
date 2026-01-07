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

pub struct PipManager {
    cache: Arc<Cache>,
    global: bool,
}

impl PipManager {
    pub fn new(cache: Arc<Cache>, global: bool) -> Self {
        Self { cache, global }
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        let cmd = if self.global { "pip3" } else { "pip" };
        let mut cmd_args = Vec::new();
        if self.global {
            cmd_args.push("--global");
        }
        cmd_args.extend_from_slice(args);

        debug!("执行 {} 命令: {}", cmd, cmd_args.join(" "));

        retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let output = timeout(COMMAND_TIMEOUT, Command::new(cmd).args(&cmd_args).output())
                .await
                .map_err(|_| BoxyError::CommandTimeout)?
                .map_err(|_| BoxyError::CommandFailed {
                    manager: "pip".to_string(),
                    command: cmd_args.join(" "),
                    exit_code: -1,
                })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(BoxyError::CommandFailed {
                    manager: "pip".to_string(),
                    command: cmd_args.join(" "),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            }
        })
        .await
    }

    fn parse_list_output(&self, output: &str) -> Vec<Package> {
        output
            .lines()
            .skip(2) // 跳过标题行
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // pip list 输出格式: package_name version
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                Some(Package {
                    name: parts[0].to_string(),
                    version: parts[1].to_string(),
                    manager: "pip".to_string(),
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
impl PackageManager for PipManager {
    fn name(&self) -> &str {
        "pip"
    }

    async fn check_available(&self) -> Result<bool> {
        let cmd = if self.global { "pip3" } else { "pip" };
        match Command::new(cmd).arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get("pip").await? {
            debug!("使用缓存的 pip 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["list"]).await?;
        let packages = self.parse_list_output(&output);

        self.cache.set("pip", &packages).await?;
        debug!("pip 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = self.exec(&["search", query]).await?;

        let packages: Vec<Package> = output
            .lines()
            .skip(2) // 跳过标题行
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // pip search 输出格式: package_name (version) - description
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    return None;
                }

                let name = parts[0].to_string();
                let version = parts
                    .get(1)
                    .map(|s| s.trim_matches(&['(', ')'] as &[char]).to_string())
                    .unwrap_or_default();
                let description = if parts.len() > 3 {
                    Some(parts[3..].join(" "))
                } else {
                    None
                };

                Some(Package {
                    name,
                    version,
                    manager: "pip".to_string(),
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
        let output = self.exec(&["show", name]).await?;

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
            manager: "pip".to_string(),
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
        let mut args: Vec<String> = vec!["install".to_string()];
        let target = match version {
            Some(v) => format!("{}=={}", name, v),
            None => name.to_string(),
        };
        args.push(target);

        info!("pip install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("pip").await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("pip install --upgrade {}", name);
        self.exec(&["install", "--upgrade", name]).await?;
        self.cache.invalidate("pip").await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["uninstall".to_string()];
        if force {
            args.push("--yes".to_string());
        }
        args.push(name.to_string());

        warn!("pip uninstall {} (force: {})", name, force);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate("pip").await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        let output = self.exec(&["list", "--outdated"]).await?;

        let outdated: Vec<Package> = output
            .lines()
            .skip(2) // 跳过标题行
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // pip list --outdated 输出格式: package_name current_version (latest_version)
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
                    manager: "pip".to_string(),
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
    fn test_pip_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PipManager::new(cache.clone(), false);
        assert_eq!(manager.name(), "pip");
    }

    #[test]
    fn test_parse_list_output() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PipManager::new(cache, false);
        let output = "Package    Version\n------------\nrequests  2.31.0\nurllib3   2.0.7\n";
        let packages = manager.parse_list_output(output);
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "requests");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PipManager::new(cache, false);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
