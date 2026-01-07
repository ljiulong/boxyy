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
use std::sync::Arc;
use std::env;

pub const MANAGER_NAMES: [&str; 10] = [
  "brew", "npm", "pnpm", "yarn", "bun", "pip", "pipx", "uv", "cargo", "mas",
];

/// 检查包管理器是否支持 global 参数
pub fn supports_global(name: &str) -> bool {
  matches!(name, "npm" | "pnpm" | "yarn" | "bun")
}

pub fn create_manager(name: &str, cache: Arc<Cache>, global: bool) -> Option<Box<dyn PackageManager>> {
  let local_workdir = if global {
    None
  } else {
    env::current_dir().ok()
  };
  match name {
    "brew" => Some(Box::new(BrewManager::new(cache))),
    "npm" => Some(Box::new(NpmManager::new(
      cache,
      if global { NpmScope::Global } else { NpmScope::Local },
      local_workdir.clone(),
    ))),
    "pnpm" => Some(Box::new(PnpmManager::new(cache, global, local_workdir.clone()))),
    "yarn" => Some(Box::new(YarnManager::new(cache, global, local_workdir.clone()))),
    "bun" => Some(Box::new(BunManager::new(cache, global, local_workdir.clone()))),
    "pip" => Some(Box::new(PipManager::new(cache, global))),
    "pipx" => Some(Box::new(PipxManager::new(cache))),
    "uv" => Some(Box::new(UvManager::new(cache, global))),
    "cargo" => Some(Box::new(CargoManager::new(cache, global))),
    "mas" => Some(Box::new(MasManager::new(cache))),
    _ => None,
  }
}
