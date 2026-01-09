use async_trait::async_trait;
use boxy_cache::Cache;
use boxy_core::{
    manager::PackageManager,
    package::{Capability, Package},
};
use boxy_error::{BoxyError, Result};
use std::{
    collections::hash_map::DefaultHasher,
    env,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_SIZE_PACKAGES: usize = 1000;

pub struct BunManager {
    cache: Arc<Cache>,
    global: bool,
    workdir: Option<PathBuf>,
    cache_key: String,
}

impl BunManager {
    pub fn new(cache: Arc<Cache>, global: bool, workdir: Option<PathBuf>) -> Self {
        let cache_key = Self::build_cache_key(global, workdir.as_ref());
        Self {
            cache,
            global,
            workdir,
            cache_key,
        }
    }

    fn build_cache_key(global: bool, workdir: Option<&PathBuf>) -> String {
        let base = if global { "bun-global" } else { "bun-local" };
        if let Some(dir) = workdir {
            let mut hasher = DefaultHasher::new();
            dir.hash(&mut hasher);
            // 使用路径哈希，避免缓存键过长
            return format!("{}-{}", base, hasher.finish());
        }
        base.to_string()
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        let mut cmd_args = Vec::new();
        if self.global {
            cmd_args.push("--global");
        }
        cmd_args.extend_from_slice(args);

        debug!("执行 bun 命令: {}", cmd_args.join(" "));

        let mut cmd = Command::new("bun");
        cmd.args(&cmd_args);
        if let Some(workdir) = &self.workdir {
            cmd.current_dir(workdir);
        }
        let output = timeout(COMMAND_TIMEOUT, cmd.output())
            .await
            .map_err(|_| BoxyError::CommandTimeout)?
            .map_err(|_| BoxyError::CommandFailed {
                manager: "bun".to_string(),
                command: cmd_args.join(" "),
                exit_code: -1,
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(BoxyError::CommandFailed {
                manager: "bun".to_string(),
                command: cmd_args.join(" "),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
    }

    fn expand_home_path(path: &str) -> Option<PathBuf> {
        if let Some(rest) = path.strip_prefix("~/") {
            let home = env::var("HOME").ok()?;
            return Some(PathBuf::from(home).join(rest));
        }
        Some(PathBuf::from(path))
    }

    async fn resolve_root(&self) -> Option<PathBuf> {
        if self.global {
            Self::expand_home_path("~/.bun/install/global/node_modules")
        } else {
            let root = match &self.workdir {
                Some(dir) => dir.join("node_modules"),
                None => env::current_dir().ok()?.join("node_modules"),
            };
            if root.exists() {
                Some(root)
            } else {
                None
            }
        }
    }

    async fn collect_sizes(
        &self,
        root: &Path,
        names: &[String],
    ) -> Result<std::collections::HashMap<String, u64>> {
        let mut items: Vec<(String, PathBuf)> = Vec::new();
        for name in names {
            let path = root.join(name);
            if path.exists() {
                items.push((name.clone(), path));
            }
        }

        if items.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        let mut sizes = std::collections::HashMap::new();
        for chunk in items.chunks(100) {
            let mut cmd = Command::new("du");
            cmd.arg("-sk");
            let mut path_map = std::collections::HashMap::new();
            for (name, path) in chunk {
                let path_str = path.to_string_lossy().to_string();
                path_map.insert(path_str.clone(), name.clone());
                cmd.arg(path_str);
            }

            let output = cmd.output().await.map_err(|_| BoxyError::CommandFailed {
                manager: "bun".to_string(),
                command: "du -sk".to_string(),
                exit_code: -1,
            })?;

            if !output.status.success() {
                return Err(BoxyError::CommandFailed {
                    manager: "bun".to_string(),
                    command: "du -sk".to_string(),
                    exit_code: output.status.code().unwrap_or(-1),
                });
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let mut parts = line.split_whitespace();
                let size_kb = parts.next().and_then(|value| value.parse::<u64>().ok());
                let size_value = match size_kb {
                    Some(value) => value,
                    None => continue,
                };
                let path_value = match parts.next() {
                    Some(value) => {
                        let prefix_len = line.find(value).unwrap_or(0);
                        line[prefix_len..].trim()
                    }
                    None => continue,
                };

                if let Some(name) = path_map.get(path_value) {
                    sizes.insert(name.clone(), size_value.saturating_mul(1024));
                }
            }
        }

        Ok(sizes)
    }
}

#[async_trait]
impl PackageManager for BunManager {
    fn name(&self) -> &str {
        "bun"
    }

    fn cache_key(&self) -> &str {
        &self.cache_key
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("bun").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get(self.cache_key()).await? {
            debug!("使用缓存的 bun 包列表");
            return Ok(cached);
        }

        // bun pm ls 列出已安装的包
        let output = self.exec(&["pm", "ls"]).await?;

        let mut packages: Vec<Package> = output
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with("bun") {
                    return None;
                }

                // 解析格式: package@version
                let parts: Vec<&str> = line.split('@').collect();
                let (name, version) = if parts.len() >= 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    (line.to_string(), String::new())
                };

                Some(Package {
                    name,
                    version,
                    manager: "bun".to_string(),
                    description: None,
                    homepage: None,
                    license: None,
                    installed_path: if self.global {
                        Some("~/.bun/install/global".to_string())
                    } else {
                        None
                    },
                    size: None,
                    outdated: false,
                    latest_version: None,
                })
            })
            .collect();

        if packages.len() <= MAX_SIZE_PACKAGES {
            if let Some(root) = self.resolve_root().await {
                let names: Vec<String> =
                    packages.iter().map(|pkg| pkg.name.clone()).collect();
                match self.collect_sizes(&root, &names).await {
                    Ok(size_map) => {
                        for pkg in packages.iter_mut() {
                            if let Some(size) = size_map.get(&pkg.name) {
                                pkg.size = Some(*size);
                                pkg.installed_path =
                                    Some(root.join(&pkg.name).to_string_lossy().to_string());
                            }
                        }
                    }
                    Err(err) => {
                        warn!("bun 获取包大小失败: {}", err);
                    }
                }
            }
        } else {
            warn!("bun 包数量过多，跳过大小统计");
        }

        self.cache.set(self.cache_key(), &packages).await?;
        debug!("bun 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // bun pm search 搜索包
        let output = self.exec(&["pm", "search", query]).await?;

        let packages: Vec<Package> = output
            .lines()
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
                let version = parts.get(1).unwrap_or(&"").to_string();

                Some(Package {
                    name,
                    version,
                    manager: "bun".to_string(),
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
        // bun pm info 获取包信息
        let output = self.exec(&["pm", "info", name]).await?;

        let mut version = String::new();
        let mut description = None;

        for line in output.lines() {
            let line = line.trim();
            if line.starts_with("version:") {
                version = line.replace("version:", "").trim().to_string();
            } else if line.starts_with("description:") {
                description = Some(line.replace("description:", "").trim().to_string());
            }
        }

        let mut package = Package {
            name: name.to_string(),
            version,
            manager: "bun".to_string(),
            description,
            homepage: None,
            license: None,
            installed_path: if self.global {
                Some("~/.bun/install/global".to_string())
            } else {
                None
            },
            size: None,
            outdated: false,
            latest_version: None,
        };

        if let Some(root) = self.resolve_root().await {
            let names = vec![package.name.clone()];
            match self.collect_sizes(&root, &names).await {
                Ok(size_map) => {
                    if let Some(size) = size_map.get(&package.name) {
                        package.size = Some(*size);
                        package.installed_path =
                            Some(root.join(&package.name).to_string_lossy().to_string());
                    }
                }
                Err(err) => {
                    warn!("bun 获取包大小失败: {}", err);
                }
            }
        }

        Ok(package)
    }

    async fn install(&self, name: &str, version: Option<&str>, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["install".to_string()];
        if force {
            args.push("--force".to_string());
        }
        let target = match version {
            Some(v) => format!("{}@{}", name, v),
            None => name.to_string(),
        };
        args.push(target);

        info!("bun install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate(self.cache_key()).await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("bun update {}", name);
        self.exec(&["update", name]).await?;
        self.cache.invalidate(self.cache_key()).await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["remove".to_string(), name.to_string()];
        if force {
            args.push("--force".to_string());
        }

        warn!("bun remove {} (force: {})", name, force);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate(self.cache_key()).await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        // bun 没有直接的 outdated 命令，需要手动检查
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
                            manager: "bun".to_string(),
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

    /// 清理 bun 缓存
    ///
    /// 由于官方命令 `bun pm cache rm` 存在已知问题，
    /// 此方法手动删除 ~/.bun/install/cache 目录
    async fn clean_cache(&self) -> Result<()> {
        info!("清理 bun 缓存目录");

        // 获取用户主目录
        let home_dir = dirs::home_dir().ok_or_else(|| BoxyError::CommandFailed {
            manager: "bun".to_string(),
            command: "clean_cache".to_string(),
            exit_code: -1,
        })?;

        let cache_dir = home_dir.join(".bun/install/cache");

        if cache_dir.exists() {
            tokio::fs::remove_dir_all(&cache_dir).await.map_err(|e| {
                BoxyError::CommandFailed {
                    manager: "bun".to_string(),
                    command: format!("删除缓存目录失败: {}", e),
                    exit_code: -1,
                }
            })?;
            info!("已删除 bun 缓存目录: {:?}", cache_dir);
        }

        Ok(())
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
    fn test_bun_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = BunManager::new(cache.clone(), true, None);
        assert_eq!(manager.name(), "bun");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = BunManager::new(cache, true, None);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
