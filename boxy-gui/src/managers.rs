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

pub fn create_manager(
  name: &str,
  cache: Arc<Cache>,
  global: bool,
  workdir: Option<PathBuf>,
) -> Option<Box<dyn PackageManager>> {
  match name {
    "brew" => Some(Box::new(BrewManager::new(cache))),
    "npm" => Some(Box::new(NpmManager::new(
      cache,
      if global { NpmScope::Global } else { NpmScope::Local },
      workdir,
    ))),
    "pnpm" => Some(Box::new(PnpmManager::new(cache, global, workdir))),
    "yarn" => Some(Box::new(YarnManager::new(cache, global, workdir))),
    "bun" => Some(Box::new(BunManager::new(cache, global, workdir))),
    "pip" => Some(Box::new(PipManager::new(cache, false))),
    "pipx" => Some(Box::new(PipxManager::new(cache))),
    "uv" => Some(Box::new(UvManager::new(cache, false))),
    "cargo" => Some(Box::new(CargoManager::new(cache, false))),
    "mas" => Some(Box::new(MasManager::new(cache))),
    _ => None,
  }
}
