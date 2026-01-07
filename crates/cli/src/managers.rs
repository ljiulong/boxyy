use boxy_brew::BrewManager;
use boxy_bun::BunManager;
use boxy_cache::Cache;
use boxy_cargo::CargoManager;
use boxy_core::manager::PackageManager;
use boxy_mas::MasManager;
use boxy_npm::{NpmManager, NpmScope};
use boxy_pip::PipManager;
use boxy_pipx::PipxManager;
use boxy_pnpm::PnpmManager;
use boxy_uv::UvManager;
use boxy_yarn::YarnManager;
use std::{path::PathBuf, sync::Arc};

pub const MANAGER_NAMES: [&str; 10] = [
    "brew", "npm", "pnpm", "yarn", "bun", "pip", "pipx", "uv", "cargo", "mas",
];

/// 创建包管理器实例
/// 
/// # 参数
/// 
/// * `name` - 包管理器名称
/// * `cache` - 缓存实例
/// * `global` - 是否使用全局范围（针对 npm、pnpm、yarn、bun）
pub fn create_manager(
    name: &str,
    cache: Arc<Cache>,
    global: bool,
    workdir: Option<&PathBuf>,
) -> Option<Box<dyn PackageManager>> {
    match name {
        "brew" => Some(Box::new(BrewManager::new(cache))),
        "npm" => Some(Box::new(NpmManager::new(
            cache,
            if global { NpmScope::Global } else { NpmScope::Local },
            workdir.cloned(),
        ))),
        "pnpm" => Some(Box::new(PnpmManager::new(cache, global, workdir.cloned()))),
        "yarn" => Some(Box::new(YarnManager::new(cache, global, workdir.cloned()))),
        "bun" => Some(Box::new(BunManager::new(cache, global, workdir.cloned()))),
        "pip" => Some(Box::new(PipManager::new(cache, false))),
        "pipx" => Some(Box::new(PipxManager::new(cache))),
        "uv" => Some(Box::new(UvManager::new(cache, false))),
        "cargo" => Some(Box::new(CargoManager::new(cache, false))),
        "mas" => Some(Box::new(MasManager::new(cache))),
        _ => None,
    }
}

/// 检查包管理器是否支持 global 参数
pub fn supports_global(name: &str) -> bool {
    matches!(name, "npm" | "pnpm" | "yarn" | "bun")
}
