use async_trait::async_trait;
use boxy_cache::Cache;
use boxy_core::{
    manager::PackageManager,
    package::{Capability, Package},
};
use boxy_core::retry::retry_with_backoff;
use boxy_core::{DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY};
use boxy_error::{BoxyError, Result};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::process::Command;
use tokio::time::{timeout, Duration};
use tracing::{debug, info, warn};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_SIZE_PACKAGES: usize = 1000;

#[derive(Debug, Deserialize)]
struct PnpmInfoOutput {
    name: String,
    version: String,
    description: String,
    homepage: Option<String>,
    license: Option<String>,
}

pub struct PnpmManager {
    cache: Arc<Cache>,
    global: bool,
    workdir: Option<PathBuf>,
    cache_key: String,
}

impl PnpmManager {
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
        let base = if global { "pnpm-global" } else { "pnpm-local" };
        if let Some(dir) = workdir {
            let mut hasher = DefaultHasher::new();
            dir.hash(&mut hasher);
            // 使用路径哈希，避免缓存键过长
            return format!("{}-{}", base, hasher.finish());
        }
        base.to_string()
    }

    fn cache_key_value(&self) -> &str {
        &self.cache_key
    }

    fn parse_dependency_version(value: &Value) -> Option<String> {
        if let Some(version) = value.as_str() {
            return Some(version.to_string());
        }
        value
            .get("version")
            .and_then(|version| version.as_str())
            .map(|version| version.to_string())
    }

    fn extract_dependencies(value: &Value) -> Vec<(String, String)> {
        let dependencies = value.get("dependencies").and_then(|deps| deps.as_object());
        let map = if let Some(deps) = dependencies {
            deps
        } else {
            match value.as_object() {
                Some(obj) => obj,
                None => return Vec::new(),
            }
        };

        map.iter()
            .filter_map(|(name, dep_value)| {
                Self::parse_dependency_version(dep_value).map(|version| (name.clone(), version))
            })
            .collect()
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        let mut cmd_args = Vec::new();
        if self.global {
            cmd_args.push("-g");
        }
        cmd_args.extend_from_slice(args);

        debug!("执行 pnpm 命令: {}", cmd_args.join(" "));

        retry_with_backoff(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY, || async {
            let mut cmd = Command::new("pnpm");
            cmd.args(&cmd_args);
            if let Some(workdir) = &self.workdir {
                cmd.current_dir(workdir);
            }
            let output = timeout(COMMAND_TIMEOUT, cmd.output())
            .await
            .map_err(|_| BoxyError::CommandTimeout)?
            .map_err(|_| BoxyError::CommandFailed {
                manager: "pnpm".to_string(),
                command: cmd_args.join(" "),
                exit_code: -1,
            })?;

            if output.status.success() {
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            } else {
                Err(BoxyError::CommandFailed {
                    manager: "pnpm".to_string(),
                    command: cmd_args.join(" "),
                    exit_code: output.status.code().unwrap_or(-1),
                })
            }
        })
        .await
    }

    async fn resolve_root(&self) -> Option<PathBuf> {
        let output = if self.global {
            self.exec(&["root", "-g"]).await.ok()?
        } else {
            self.exec(&["root"]).await.ok()?
        };
        let root = output.lines().next()?.trim();
        if root.is_empty() {
            None
        } else {
            Some(PathBuf::from(root))
        }
    }

    async fn collect_sizes(&self, root: &Path, names: &[String]) -> Result<HashMap<String, u64>> {
        let mut items: Vec<(String, PathBuf)> = Vec::new();
        for name in names {
            let path = root.join(name);
            if path.exists() {
                items.push((name.clone(), path));
            }
        }

        if items.is_empty() {
            return Ok(HashMap::new());
        }

        let mut sizes = HashMap::new();
        for chunk in items.chunks(100) {
            let mut cmd = Command::new("du");
            cmd.arg("-sk");
            let mut path_map = HashMap::new();
            for (name, path) in chunk {
                let path_str = path.to_string_lossy().to_string();
                path_map.insert(path_str.clone(), name.clone());
                cmd.arg(path_str);
            }

            let output = cmd.output().await.map_err(|_| BoxyError::CommandFailed {
                manager: "pnpm".to_string(),
                command: "du -sk".to_string(),
                exit_code: -1,
            })?;

            if !output.status.success() {
                return Err(BoxyError::CommandFailed {
                    manager: "pnpm".to_string(),
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
impl PackageManager for PnpmManager {
    fn name(&self) -> &str {
        "pnpm"
    }

    fn cache_key(&self) -> &str {
        self.cache_key_value()
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("pnpm").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        let cache_key = self.cache_key_value();
        if let Some(cached) = self.cache.get(cache_key).await? {
            debug!("使用缓存的 pnpm 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["list", "--json", "--depth=0"]).await?;

        let root: Value = serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
            message: format!("解析 pnpm list 输出失败: {}", e),
        })?;

        let deps = if let Some(list) = root.as_array() {
            list.iter()
                .flat_map(Self::extract_dependencies)
                .collect::<Vec<_>>()
        } else {
            Self::extract_dependencies(&root)
        };

        let mut packages: Vec<Package> = deps
            .into_iter()
            .map(|(name, version)| Package {
                name,
                version: version.trim_start_matches(['^', '~']).to_string(),
                manager: "pnpm".to_string(),
                description: None,
                homepage: None,
                license: None,
                installed_path: if self.global {
                    Some("~/.pnpm-global".to_string())
                } else {
                    None
                },
                size: None,
                outdated: false,
                latest_version: None,
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
                        warn!("pnpm 获取包大小失败: {}", err);
                    }
                }
            }
        } else {
            warn!("pnpm 包数量过多，跳过大小统计");
        }

        self.cache.set(cache_key, &packages).await?;
        debug!("pnpm 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        // pnpm 使用 npm 的搜索
        let output = self.exec(&["search", "--json", query]).await?;

        let data: serde_json::Value =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 pnpm search 输出失败: {}", e),
            })?;

        let packages = if let Some(array) = data.as_array() {
            array
                .iter()
                .filter_map(|item| {
                    let name = item["name"].as_str()?.to_string();
                    let version = item["version"].as_str()?.to_string();
                    let description = item["description"].as_str().map(|s| s.to_string());

                    Some(Package {
                        name,
                        version,
                        manager: "pnpm".to_string(),
                        description,
                        homepage: None,
                        license: None,
                        installed_path: None,
                        size: None,
                        outdated: false,
                        latest_version: None,
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

        Ok(packages)
    }

    async fn get_info(&self, name: &str) -> Result<Package> {
        let output = self.exec(&["info", "--json", name]).await?;

        let info: PnpmInfoOutput =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 pnpm info 输出失败: {}", e),
            })?;

        let mut package = Package {
            name: info.name.clone(),
            version: info.version.clone(),
            manager: "pnpm".to_string(),
            description: Some(info.description),
            homepage: info.homepage,
            license: info.license,
            installed_path: if self.global {
                Some("~/.pnpm-global".to_string())
            } else {
                None
            },
            size: None,
            outdated: false,
            latest_version: Some(info.version),
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
                    warn!("pnpm 获取包大小失败: {}", err);
                }
            }
        }

        Ok(package)
    }

    async fn install(&self, name: &str, version: Option<&str>) -> Result<()> {
        let mut args: Vec<String> = vec!["install".to_string()];
        let target = match version {
            Some(v) => format!("{}@{}", name, v),
            None => name.to_string(),
        };
        args.push(target);

        info!("pnpm install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate(self.cache_key_value()).await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        info!("pnpm update {}", name);
        self.exec(&["update", name]).await?;
        self.cache.invalidate(self.cache_key_value()).await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["uninstall".to_string(), name.to_string()];
        if force {
            args.push("--force".to_string());
        }

        warn!("pnpm uninstall {} (force: {})", name, force);

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate(self.cache_key_value()).await?;

        Ok(())
    }

    async fn check_outdated(&self) -> Result<Vec<Package>> {
        let output = self.exec(&["outdated", "--json"]).await?;

        if output.trim().is_empty() {
            return Ok(Vec::new());
        }

        let outdated: HashMap<String, PnpmOutdatedPackage> = serde_json::from_str(&output)
            .map_err(|e| BoxyError::JsonError {
                message: format!("解析 pnpm outdated 输出失败: {}", e),
            })?;

        let packages = outdated
            .into_iter()
            .map(|(name, pkg)| Package {
                name,
                version: pkg.current,
                manager: "pnpm".to_string(),
                description: None,
                homepage: None,
                license: None,
                installed_path: if self.global {
                    Some("~/.pnpm-global".to_string())
                } else {
                    None
                },
                size: None,
                outdated: true,
                latest_version: Some(pkg.latest),
            })
            .collect();

        Ok(packages)
    }

    fn capabilities(&self) -> &[Capability] {
        use Capability::*;

        &[
            ListInstalled,
            SearchRemote,
            QueryDependencies,
            VersionSelection,
        ]
    }
}

#[derive(Debug, Deserialize)]
struct PnpmOutdatedPackage {
    current: String,
    latest: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pnpm_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PnpmManager::new(cache.clone(), true, None);
        assert_eq!(manager.name(), "pnpm");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = PnpmManager::new(cache, true, None);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::QueryDependencies));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
