use async_trait::async_trait;
use boxy_cache::Cache;
use boxy_core::{
    manager::PackageManager,
    package::{Capability, Package},
};
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
struct NpmListOutput {
    dependencies: Option<NpmDependencies>,
}

type NpmDependencies = HashMap<String, NpmDependency>;

#[derive(Debug, Deserialize)]
struct NpmDependency {
    version: String,
}

#[derive(Debug, Deserialize)]
struct NpmInfoOutput {
    name: String,
    version: String,
    description: String,
    homepage: Option<String>,
    license: Option<String>,
}

pub struct NpmManager {
    cache: Arc<Cache>,
    scope: NpmScope,
    workdir: Option<PathBuf>,
    cache_key: String,
}

impl NpmManager {
    pub fn new(cache: Arc<Cache>, scope: NpmScope, workdir: Option<PathBuf>) -> Self {
        let cache_key = Self::build_cache_key(scope, workdir.as_ref());
        Self {
            cache,
            scope,
            workdir,
            cache_key,
        }
    }

    fn build_cache_key(scope: NpmScope, workdir: Option<&PathBuf>) -> String {
        let base = match scope {
            NpmScope::Global => "npm-global",
            NpmScope::Local => "npm-local",
        };

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

    fn parse_search_item(value: &Value) -> Option<Package> {
        let package_value = value.get("package").unwrap_or(value);
        let name = package_value.get("name")?.as_str()?.to_string();
        let version = package_value.get("version")?.as_str()?.to_string();
        let description = package_value
            .get("description")
            .and_then(|desc| desc.as_str())
            .map(|desc| desc.to_string());

        Some(Package {
            name,
            version,
            manager: "npm".to_string(),
            description,
            homepage: None,
            license: None,
            installed_path: None,
            size: None,
            outdated: false,
            latest_version: None,
        })
    }

    fn parse_search_packages(value: &Value) -> Vec<Package> {
        if let Some(list) = value.as_array() {
            return list
                .iter()
                .filter_map(Self::parse_search_item)
                .collect();
        }

        if let Some(objects) = value.get("objects").and_then(|list| list.as_array()) {
            return objects
                .iter()
                .filter_map(Self::parse_search_item)
                .collect();
        }

        if let Some(results) = value.get("results").and_then(|list| list.as_array()) {
            return results
                .iter()
                .filter_map(Self::parse_search_item)
                .collect();
        }

        Vec::new()
    }

    async fn exec(&self, args: &[&str]) -> Result<String> {
        let mut cmd_args = Vec::new();
        if self.scope == NpmScope::Global {
            cmd_args.push("-g");
        }
        cmd_args.extend_from_slice(args);

        debug!("执行 npm 命令: {}", cmd_args.join(" "));

        let mut cmd = Command::new("npm");
        cmd.args(&cmd_args);
        if let Some(workdir) = &self.workdir {
            cmd.current_dir(workdir);
        }
        let output = timeout(COMMAND_TIMEOUT, cmd.output())
            .await
            .map_err(|_| BoxyError::CommandTimeout)?
            .map_err(|_| BoxyError::CommandFailed {
                manager: "npm".to_string(),
                command: cmd_args.join(" "),
                exit_code: -1,
            })?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(BoxyError::CommandFailed {
                manager: "npm".to_string(),
                command: cmd_args.join(" "),
                exit_code: output.status.code().unwrap_or(-1),
            })
        }
    }

    async fn resolve_root(&self) -> Option<PathBuf> {
        let output = if self.scope == NpmScope::Global {
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
                manager: "npm".to_string(),
                command: "du -sk".to_string(),
                exit_code: -1,
            })?;

            if !output.status.success() {
                return Err(BoxyError::CommandFailed {
                    manager: "npm".to_string(),
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
impl PackageManager for NpmManager {
    fn name(&self) -> &str {
        "npm"
    }

    async fn check_available(&self) -> Result<bool> {
        match Command::new("npm").arg("--version").output().await {
            Ok(output) => Ok(output.status.success()),
            Err(_) => Ok(false),
        }
    }

    async fn list_installed(&self) -> Result<Vec<Package>> {
        if let Some(cached) = self.cache.get(self.cache_key_value()).await? {
            debug!("使用缓存的 npm 包列表");
            return Ok(cached);
        }

        let output = self.exec(&["list", "--json"]).await?;

        let data: NpmListOutput =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 npm list 输出失败: {}", e),
            })?;

        let mut packages: Vec<Package> = data
            .dependencies
            .unwrap_or_default()
            .into_iter()
            .map(|(name, dep)| Package {
                name,
                version: dep.version.trim_start_matches(['^', '~']).to_string(),
                manager: "npm".to_string(),
                description: None,
                homepage: None,
                license: None,
                installed_path: if self.scope == NpmScope::Global {
                    Some("/usr/local/lib/node_modules".to_string())
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
                        warn!("npm 获取包大小失败: {}", err);
                    }
                }
            }
        } else {
            warn!("npm 包数量过多，跳过大小统计");
        }

        self.cache.set(self.cache_key_value(), &packages).await?;
        debug!("npm 已安装包: {} 个", packages.len());

        Ok(packages)
    }

    async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let output = self.exec(&["search", "--json", query]).await?;

        let data: Value = serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
            message: format!("解析 npm search 输出失败: {}", e),
        })?;

        Ok(Self::parse_search_packages(&data))
    }

    async fn get_info(&self, name: &str) -> Result<Package> {
        let output = self.exec(&["info", "--json", name]).await?;

        let info: NpmInfoOutput =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 npm info 输出失败: {}", e),
            })?;

        let mut package = Package {
            name: info.name.clone(),
            version: info.version.clone(),
            manager: "npm".to_string(),
            description: Some(info.description),
            homepage: info.homepage,
            license: info.license,
            installed_path: if self.scope == NpmScope::Global {
                Some("/usr/local/lib/node_modules".to_string())
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
                    warn!("npm 获取包大小失败: {}", err);
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

        info!("npm install {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate(self.cache_key_value()).await?;

        Ok(())
    }

    async fn upgrade(&self, name: &str) -> Result<()> {
        let args: Vec<String> = vec!["update".to_string(), name.to_string()];

        info!("npm update {}", args.join(" "));

        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        self.exec(&args_refs).await?;
        self.cache.invalidate(self.cache_key_value()).await?;

        Ok(())
    }

    async fn uninstall(&self, name: &str, force: bool) -> Result<()> {
        let mut args: Vec<String> = vec!["uninstall".to_string(), name.to_string()];
        if force {
            args.push("-f".to_string());
        }

        warn!("npm uninstall {} (force: {})", name, force);

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

        let outdated: HashMap<String, NpmOutdatedPackage> =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 npm outdated 输出失败: {}", e),
            })?;

        let packages = outdated
            .into_iter()
            .map(|(name, pkg)| Package {
                name,
                version: pkg.current,
                manager: "npm".to_string(),
                description: None,
                homepage: None,
                license: None,
                installed_path: if self.scope == NpmScope::Global {
                    Some("/usr/local/lib/node_modules".to_string())
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

    async fn list_dependencies(&self, name: &str) -> Result<Vec<Package>> {
        let output = self
            .exec(&["view", name, "dependencies", "--json"])
            .await?;

        if output.trim().is_empty() || output.trim() == "null" {
            return Ok(Vec::new());
        }

        let deps: HashMap<String, String> =
            serde_json::from_str(&output).map_err(|e| BoxyError::JsonError {
                message: format!("解析 npm dependencies 输出失败: {}", e),
            })?;

        let packages = deps
            .into_iter()
            .map(|(dep_name, dep_version)| Package {
                name: dep_name,
                version: dep_version.trim_start_matches(['^', '~']).to_string(),
                manager: "npm".to_string(),
                description: None,
                homepage: None,
                license: None,
                installed_path: None,
                size: None,
                outdated: false,
                latest_version: None,
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

    fn cache_key(&self) -> &str {
        self.cache_key_value()
    }
}

#[derive(Debug, Deserialize)]
struct NpmOutdatedPackage {
    current: String,
    latest: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NpmScope {
    Global,
    Local,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_npm_manager_creation() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = NpmManager::new(cache.clone(), NpmScope::Global, None);
        assert_eq!(manager.name(), "npm");
    }

    #[test]
    fn test_capabilities() {
        let cache = Arc::new(Cache::new().unwrap());
        let manager = NpmManager::new(cache, NpmScope::Global, None);
        let caps = manager.capabilities();

        assert!(caps.contains(&Capability::ListInstalled));
        assert!(caps.contains(&Capability::SearchRemote));
        assert!(caps.contains(&Capability::QueryDependencies));
        assert!(caps.contains(&Capability::VersionSelection));
    }
}
