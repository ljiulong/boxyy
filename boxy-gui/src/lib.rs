use boxy_cache::Cache;
use boxy_core::ManagerExecutor;
use std::collections::HashMap;
#[cfg(target_os = "macos")]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::Arc;
use tauri::Wry;
use tracing::info;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

mod commands;
mod logging;
mod managers;

pub struct TaskStore {
  pub tasks: Vec<boxy_core::Job>,
  pub logs: HashMap<String, Vec<String>>,
  pub handles: HashMap<String, JoinHandle<()>>,
}

pub struct AppState {
  pub cache: Arc<Cache>,
  pub tasks: Arc<Mutex<TaskStore>>,
  pub executor: Arc<ManagerExecutor>,
}

pub fn build() -> tauri::Builder<Wry> {
  // 初始化日志系统，失败时继续运行但输出错误
  if let Err(err) = logging::init_logging() {
    eprintln!("初始化日志失败: {}", err);
  }
  info!("Boxy GUI 启动");
  ensure_macos_path();

  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .plugin(tauri_plugin_process::init())
    .manage(AppState {
      cache: Arc::new(Cache::new().expect("cache")),
      tasks: Arc::new(Mutex::new(TaskStore::new())),
      executor: Arc::new(ManagerExecutor::new(1, std::time::Duration::from_secs(1))),
    })
    .invoke_handler(tauri::generate_handler![
      commands::scan_managers,
      commands::get_manager_packages,
      commands::refresh_manager,
      commands::install_package,
      commands::update_package,
      commands::update_outdated_packages,
      commands::uninstall_package,
      commands::search_packages,
      commands::get_package_info,
      commands::get_tasks,
      commands::get_task_logs,
      commands::cancel_task,
      commands::open_external_url,
      commands::delete_task,
      commands::clear_tasks,
      commands::get_app_logs,
      commands::append_frontend_log,
      commands::get_app_log_path,
    ])
}

fn ensure_macos_path() {
  #[cfg(target_os = "macos")]
  {
    let current = env::var("PATH").unwrap_or_default();
    let mut seen: HashSet<String> = HashSet::new();
    let mut parts: Vec<String> = Vec::new();

    let mut add_path = |value: &str| {
      let trimmed = value.trim();
      if trimmed.is_empty() {
        return;
      }
      if seen.insert(trimmed.to_string()) {
        parts.push(trimmed.to_string());
      }
    };

    for entry in current.split(':') {
      add_path(entry);
    }

    if let Ok(shell_path) = resolve_login_shell_path() {
      for entry in shell_path.split(':') {
        add_path(entry);
      }
    }

    if let Some(home) = dirs::home_dir() {
      let candidates = [
        home.join(".cargo/bin"),
        home.join(".local/bin"),
        home.join(".pyenv/shims"),
        home.join(".pyenv/bin"),
        home.join(".volta/bin"),
        home.join(".npm-global/bin"),
        home.join("Library/pnpm"),
      ];
      for candidate in candidates {
        if candidate.is_dir() {
          add_path(&candidate.to_string_lossy());
        }
      }
    }

    for candidate in [
      "/opt/homebrew/bin",
      "/opt/homebrew/sbin",
      "/usr/local/bin",
      "/usr/local/sbin",
      "/usr/bin",
      "/bin",
      "/usr/sbin",
      "/sbin",
    ] {
      add_path(candidate);
    }

    if !parts.is_empty() {
      env::set_var("PATH", parts.join(":"));
    }
  }
}

#[cfg(target_os = "macos")]
fn resolve_login_shell_path() -> Result<String, String> {
  let output = Command::new("/bin/zsh")
    .arg("-lc")
    .arg("printf %s \"$PATH\"")
    .output()
    .map_err(|e| format!("无法读取 shell PATH: {}", e))?;
  if !output.status.success() {
    return Err("读取 shell PATH 失败".to_string());
  }
  let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
  if value.is_empty() {
    return Err("shell PATH 为空".to_string());
  }
  Ok(value)
}
