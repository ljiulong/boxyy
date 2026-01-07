use boxy_cache::Cache;
use boxy_core::ManagerExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Wry;
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

  tauri::Builder::default()
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_opener::init())
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
