use async_trait::async_trait;
use boxy_cache::Cache;
use boxy_core::{
    manager::PackageManager,
    package::{Capability, Package},
};
use boxy_error::{BoxyError, Result};
use std::sync::Arc;
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);

pub struct MasManager {
    cache: Arc<Cache>,
}

impl MasManager {
    pub fn new(cache: Arc<Cache>) -> Self {
        Self { cache }
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        debug!("执行 mas 命令: {}", args.join(" "));

        let output = timeout(COMMAND_TIMEOUT, Command::new("mas").args(args).output())
            .await
            .map_err(|_| BoxyError::CommandTimeout)?
            .map_err(|_| BoxyError::CommandFailed {
                manager: "mas".to_string(),
                command: args.join(" "),
                exit_code: -1,
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(BoxyError::CommandFailed {
                manager: "mas".to_string(),
                command: args.join(" "),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
    }
}

#[async_trait]
impl PackageManager for MasManager {
    fn name(&self) -> &str {
        "mas"
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("mas").arg("version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get("mas").await? {
            debug!("使用缓存的 mas 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["list"]).await?;

        let packages: Vec<Package> = output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // mas list 输出格式: ID Name (version)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let id = parts[0].to_string();
                let name = parts[1..]
                    .iter()
                    .take_while(|s| !s.starts_with('('))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                let version = parts
                    .iter()
                    .find(|s| s.starts_with('('))
                    .map(|s| s.trim_matches(&['(', ')'] as &[char]).to_string())
                    .unwrap_or_default();

                Some(Package {
                    name: if name.is_empty() { id.clone() } else { name },
                    version,
                    manager: "mas".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: Some("/Applications".to_string()),
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect();

        self.cache.set("mas", &packages).await?;
        debug!("mas 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = self.exec(&["search", query]).await?;

        let packages: Vec<Package> = output
            .lines()
            .skip(1) // 跳过标题行
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // mas search 输出格式: ID Name
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let id = parts[0].to_string();
                let name = parts[1..].join(" ");

                Some(Package {
                    name: if name.is_empty() { id.clone() } else { name },
                    version: String::new(),
                    manager: "mas".to_string(),
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
        // mas info 需要 ID，这里简化处理
        let output = self.exec(&["info", name]).await?;

        let mut version = String::new();
        let mut description = None;

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("Version:") {
                version = line.replace("Version:", "").trim().to_string();
            } else if line.starts_with("Description:") {
                description = Some(line.replace("Description:", "").trim().to_string());
            }
        }

        Ok(Package {
            name: name.to_string(),
            version,
            manager: "mas".to_string(),
            description,
            homepage: None,
            license: None,
            installed_path: Some("/Applications".to_string()),
            size: None,
            outdated: false,
            latest_version: None,
        })
    }

    async fn install(&self, name: &str, _version: Option<&str>, _force: bool) -> Result<()> {
        // mas install 使用 ID
        info!("mas install {}", name);
        self.exec(&["install", name]).await?;
        self.cache.invalidate("mas").await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("mas upgrade {}", name);
        self.exec(&["upgrade", name]).await?;
        self.cache.invalidate("mas").await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        warn!("mas uninstall {} (force: {})", name, force);
        // mas 没有直接的 uninstall，需要通过系统卸载
        // 这里只是标记缓存失效
        self.cache.invalidate("mas").await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        let output = self.exec(&["outdated"]).await?;

        let outdated: Vec<Package> = output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }

                // mas outdated 输出格式: ID Name (current) -> (latest)
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 {
                    return None;
                }

                let name = parts[1..]
                    .iter()
                    .take_while(|s| !s.starts_with('('))
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                let current_version = parts
                    .iter()
                    .find(|s| s.starts_with('('))
                    .map(|s| s.trim_matches(&['(', ')'] as &[char]).to_string())
                    .unwrap_or_default();
                let latest_version = parts
                    .iter()
                    .find(|s| s.contains("->"))
                    .and_then(|_| {
                        parts
                            .iter()
                            .skip_while(|s| !s.contains("->"))
                            .nth(1)
                            .map(|s| s.trim_matches(&['(', ')'] as &[char]).to_string())
                    })
                    .unwrap_or_default();

                Some(Package {
                    name: if name.is_empty() {
                        parts[0].to_string()
                    } else {
                        name
                    },
                    version: current_version,
                    manager: "mas".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: Some("/Applications".to_string()),
                    size: None,
                    outdated: true,
                    latest_version: if latest_version.is_empty() {
                        None
                    } else {
                        Some(latest_version)
                    },
                })
            })
            .collect();

        Ok(outdated)
    }

    fn capabilities(&self) -> &[Capability] {
        use Capability::*;

        &[ListInstalled, SearchRemote]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mas_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = MasManager::new(cache);
        assert_eq!(manager.name(), "mas");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = MasManager::new(cache);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
    }
}
