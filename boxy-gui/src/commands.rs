use crate::logging;
use crate::managers::{create_manager, MANAGER_NAMES};
use crate::{AppState, TaskStore};
use boxy_cache::Cache;
use boxy_core::{Job, JobStatus, ManagerStatus, Operation, Package};
use tauri_plugin_opener::OpenerExt;
use chrono::Utc;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::time::{interval, timeout, Duration};
use uuid::Uuid;

fn resolve_scope(
  scope: Option<String>,
  directory: Option<String>,
) -> Result<(bool, Option<PathBuf>), String> {
  let scope_value = scope.unwrap_or_else(|| "global".to_string());
  let scope_value = scope_value.to_lowercase();
  match scope_value.as_str() {
    "global" => Ok((true, None)),
    "local" => {
      let dir = directory
        .unwrap_or_default()
        .trim()
        .to_string();
      if dir.is_empty() {
        return Err("缺少目录参数".to_string());
      }
      let path = if let Some(rest) = dir.strip_prefix("~/") {
        let home = std::env::var("HOME").map_err(|_| "无法解析用户目录".to_string())?;
        PathBuf::from(home).join(rest)
      } else {
        PathBuf::from(dir)
      };
      if !path.is_dir() {
        return Err("目录不存在或不可访问".to_string());
      }
      Ok((false, Some(path)))
    }
    _ => Err("不支持的检测范围".to_string()),
  }
}

#[tauri::command]
pub async fn scan_managers(state: State<'_, AppState>) -> Result<Vec<ManagerStatus>, String> {
  let cache = state.cache.clone();
  let tasks: Vec<_> = MANAGER_NAMES
    .iter()
    .map(|name| {
      let cache = cache.clone();
      let manager_name = name.to_string();
      tokio::spawn(async move {
        let manager = create_manager(&manager_name, cache.clone(), true, None);
        if let Some(mgr) = manager {
          let available = mgr.check_available().await.unwrap_or(false);
          let cached_packages: Vec<Package> = cache
            .get(mgr.cache_key())
            .await
            .unwrap_or(None)
            .unwrap_or_default();
          let outdated_count = cached_packages.iter().filter(|pkg| pkg.outdated).count();
          ManagerStatus {
            name: manager_name,
            version: String::new(),
            available,
            package_count: cached_packages.len(),
            outdated_count,
          }
        } else {
          ManagerStatus {
            name: manager_name,
            version: String::new(),
            available: false,
            package_count: 0,
            outdated_count: 0,
          }
        }
      })
    })
    .collect();

  let mut managers = Vec::new();
  for task in tasks {
    if let Ok(status) = task.await {
      managers.push(status);
    }
  }

  Ok(managers)
}

#[tauri::command]
pub async fn get_manager_packages(
  manager: String,
  scope: Option<String>,
  directory: Option<String>,
  force: Option<bool>,
  state: State<'_, AppState>,
) -> Result<Vec<Package>, String> {
  let cache = state.cache.clone();
  if force.unwrap_or(false) {
    let (global, workdir) = resolve_scope(scope.clone(), directory.clone())?;
    let manager_impl = create_manager(&manager, cache.clone(), global, workdir)
      .ok_or_else(|| "unknown manager".to_string())?;
    cache
      .invalidate(manager_impl.cache_key())
      .await
      .map_err(|e| e.to_string())?;
  }
  let (packages, outdated_map) =
    fetch_packages(&manager, cache, scope, directory).await.map_err(|e| e.to_string())?;

  let mut packages = packages;
  for pkg in packages.iter_mut() {
    if let Some(latest) = outdated_map.get(&pkg.name) {
      pkg.outdated = true;
      pkg.latest_version = latest.clone();
    }
  }

  Ok(packages)
}

#[tauri::command]
pub async fn refresh_manager(
  manager: String,
  scope: Option<String>,
  directory: Option<String>,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let (global, workdir) = resolve_scope(scope, directory)?;
  let manager_impl = create_manager(&manager, state.cache.clone(), global, workdir)
    .ok_or_else(|| "unknown manager".to_string())?;
  if !manager_impl.check_available().await.unwrap_or(false) {
    return Ok(());
  }
  state
    .cache
    .invalidate(manager_impl.cache_key())
    .await
    .map_err(|e| e.to_string())?;
  manager_impl
    .list_installed()
    .await
    .map_err(|e| e.to_string())?;
  Ok(())
}

#[tauri::command]
pub async fn search_packages(
  query: String,
  manager: Option<String>,
  state: State<'_, AppState>,
) -> Result<Vec<Package>, String> {
  let manager_list: Vec<String> = if let Some(manager) = manager {
    vec![manager]
  } else {
    MANAGER_NAMES.iter().map(|name| name.to_string()).collect()
  };

  let cache = state.cache.clone();
  let tasks: Vec<_> = manager_list
    .iter()
    .map(|name| {
      let cache = cache.clone();
      let manager_name = name.to_string();
      let query = query.clone();
      tokio::spawn(async move {
        let manager = create_manager(&manager_name, cache.clone(), true, None);
        if let Some(mgr) = manager {
          if !mgr.check_available().await.unwrap_or(false) {
            return Vec::new();
          }
          mgr.search(&query).await.unwrap_or_default()
        } else {
          Vec::new()
        }
      })
    })
    .collect();

  let mut results = Vec::new();
  for task in tasks {
    if let Ok(mut packages) = task.await {
      results.append(&mut packages);
    }
  }

  Ok(results)
}

#[tauri::command]
pub async fn get_package_info(
  manager: String,
  package: String,
  scope: Option<String>,
  directory: Option<String>,
  state: State<'_, AppState>,
) -> Result<Package, String> {
  let (global, workdir) = resolve_scope(scope, directory)?;
  let manager_impl = create_manager(&manager, state.cache.clone(), global, workdir)
    .ok_or_else(|| "unknown manager".to_string())?;

  manager_impl
    .get_info(&package)
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn install_package(
  manager: String,
  package: String,
  version: Option<String>,
  scope: Option<String>,
  directory: Option<String>,
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<String, String> {
  let (global, workdir) = resolve_scope(scope, directory)?;
  spawn_task(
    app,
    state.inner(),
    manager,
    Operation::Install,
    package,
    version,
    false,
    global,
    workdir,
  )
  .await
}

#[tauri::command]
pub async fn update_package(
  manager: String,
  package: String,
  scope: Option<String>,
  directory: Option<String>,
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<String, String> {
  let (global, workdir) = resolve_scope(scope, directory)?;
  spawn_task(
    app,
    state.inner(),
    manager,
    Operation::Update,
    package,
    None,
    false,
    global,
    workdir,
  )
  .await
}

#[tauri::command]
pub async fn update_outdated_packages(
  manager: String,
  scope: Option<String>,
  directory: Option<String>,
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<String, String> {
  let (global, workdir) = resolve_scope(scope, directory)?;
  spawn_batch_update(app, state.inner(), manager, global, workdir).await
}

#[tauri::command]
pub async fn uninstall_package(
  manager: String,
  package: String,
  force: bool,
  scope: Option<String>,
  directory: Option<String>,
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<String, String> {
  let (global, workdir) = resolve_scope(scope, directory)?;
  spawn_task(
    app,
    state.inner(),
    manager,
    Operation::Uninstall,
    package,
    None,
    force,
    global,
    workdir,
  )
  .await
}

#[tauri::command]
pub async fn get_tasks(state: State<'_, AppState>) -> Result<Vec<Job>, String> {
  let store = state.tasks.lock().await;
  Ok(store.tasks.clone())
}

#[tauri::command]
pub async fn get_task_logs(
  task_id: String,
  state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
  let store = state.tasks.lock().await;
  Ok(store.logs.get(&task_id).cloned().unwrap_or_default())
}

#[tauri::command]
pub async fn cancel_task(
  task_id: String,
  app: AppHandle,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let mut store = state.tasks.lock().await;
  if let Some(handle) = store.handles.remove(&task_id) {
    handle.abort();
  }

  // 获取任务的管理器名称
  let manager = store
    .tasks
    .iter()
    .find(|job| job.id == task_id)
    .map(|job| job.manager.clone());

  if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id) {
    job.status = JobStatus::Canceled;
    job.finished_at = Some(Utc::now());
    job.progress = Some(100.0);
    job.step = Some("canceled".to_string());
  }

  if let Some(logs) = store.logs.get_mut(&task_id) {
    logs.push("Canceled".to_string());
  }

  let _ = app.emit("task-progress", &serde_json::json!({
    "taskId": task_id,
    "progress": 100
  }));
  let _ = app.emit("task-complete", &serde_json::json!({
    "id": task_id,
    "status": "Canceled",
    "manager": manager
  }));

  Ok(())
}

#[tauri::command]
pub async fn open_external_url(
  url: String,
  app: AppHandle,
) -> Result<(), String> {
  if url.trim().is_empty() {
    return Err("链接为空".to_string());
  }
  app
    .opener()
    .open_url(url, None::<String>)
    .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn delete_task(
  task_id: String,
  state: State<'_, AppState>,
) -> Result<(), String> {
  let mut store = state.tasks.lock().await;
  if let Some(job) = store.tasks.iter().find(|job| job.id == task_id) {
    if job.status == JobStatus::Running {
      return Err("任务运行中，无法删除".to_string());
    }
  }

  if let Some(index) = store.tasks.iter().position(|job| job.id == task_id) {
    store.tasks.remove(index);
    store.logs.remove(&task_id);
    store.handles.remove(&task_id);
    Ok(())
  } else {
    Err("任务不存在".to_string())
  }
}

#[tauri::command]
pub async fn clear_tasks(state: State<'_, AppState>) -> Result<(), String> {
  let mut store = state.tasks.lock().await;
  let running_ids: Vec<String> = store
    .tasks
    .iter()
    .filter(|job| job.status == JobStatus::Running)
    .map(|job| job.id.clone())
    .collect();

  store.tasks.retain(|job| job.status == JobStatus::Running);
  store.logs.retain(|task_id, _| running_ids.contains(task_id));
  store.handles.retain(|task_id, handle| {
    running_ids.contains(task_id) && !handle.is_finished()
  });
  Ok(())
}

#[tauri::command]
pub async fn get_app_logs() -> Result<Vec<String>, String> {
  logging::read_logs()
}

#[tauri::command]
pub async fn append_frontend_log(level: String, message: String) -> Result<(), String> {
  logging::append_frontend_log(&level, &message)
}

#[tauri::command]
pub async fn get_app_log_path() -> Result<String, String> {
  logging::log_path_display()
}

async fn fetch_packages(
  manager_name: &str,
  cache: Arc<Cache>,
  scope: Option<String>,
  directory: Option<String>,
) -> Result<(Vec<Package>, HashMap<String, Option<String>>), boxy_error::BoxyError> {
  let (global, workdir) = resolve_scope(scope, directory)
    .map_err(|message| boxy_error::BoxyError::CacheError { message })?;
  let manager = create_manager(manager_name, cache.clone(), global, workdir)
    .ok_or_else(|| boxy_error::BoxyError::ManagerNotFound {
      name: manager_name.to_string(),
    })?;

  if !manager.check_available().await.unwrap_or(false) {
    return Ok((Vec::new(), HashMap::new()));
  }

  let packages = manager.list_installed().await?;
  let outdated = match timeout(Duration::from_secs(5), manager.check_outdated()).await {
    Ok(Ok(list)) => list,
    Ok(Err(err)) => return Err(err),
    Err(_) => return Err(boxy_error::BoxyError::CommandTimeout),
  };

  let mut outdated_map = HashMap::new();
  for pkg in outdated {
    outdated_map.insert(pkg.name, pkg.latest_version);
  }

  Ok((packages, outdated_map))
}

async fn spawn_task(
  app: AppHandle,
  state: &AppState,
  manager: String,
  operation: Operation,
  package: String,
  version: Option<String>,
  force: bool,
  global: bool,
  workdir: Option<PathBuf>,
) -> Result<String, String> {
  let task_id = Uuid::new_v4().to_string();
  let job = Job {
    id: task_id.clone(),
    manager: manager.clone(),
    operation: operation.clone(),
    target: package.clone(),
    status: JobStatus::Running,
    progress: Some(0.0),
    step: Some("started".to_string()),
    started_at: Some(Utc::now()),
    finished_at: None,
    logs: Vec::new(),
    error: None,
  };

  {
    let mut store = state.tasks.lock().await;
    store.tasks.push(job.clone());
    store.logs.insert(task_id.clone(), Vec::new());
  }

  let _ = app.emit("task-progress", &serde_json::json!({
    "taskId": task_id,
    "progress": 0
  }));

  let cache = state.cache.clone();
  let tasks = state.tasks.clone();
  let executor = state.executor.clone();
  let task_id_for_worker = task_id.clone();
  let manager_for_worker = manager.clone();
  let task_handle = tokio::spawn(async move {
    let manager_impl = create_manager(&manager, cache.clone(), global, workdir.clone());
    let cache_key = manager_impl
      .as_ref()
      .map(|mgr| mgr.cache_key().to_string())
      .unwrap_or_else(|| manager.clone());
    let operation_task = executor.execute(&manager, || async {
      let manager_impl = create_manager(&manager, cache.clone(), global, workdir.clone());
      if let Some(mgr) = manager_impl {
        match operation {
          Operation::Install => mgr.install(&package, version.as_deref(), force).await,
          Operation::Update => mgr.upgrade(&package).await,
          Operation::Uninstall => {
            // 执行卸载
            mgr.uninstall(&package, force).await?;
            // 自动清理缓存（忽略错误，不中断卸载）
            let _ = mgr.clean_cache().await;
            Ok(())
          },
        }
      } else {
        Err(boxy_error::BoxyError::ManagerNotFound {
          name: manager.clone(),
        })
      }
    });

    let mut progress: f64 = 0.0;
    let mut ticker = interval(Duration::from_secs(1));
    tokio::pin!(operation_task);

    let result = loop {
      tokio::select! {
        output = &mut operation_task => break output,
        _ = ticker.tick() => {
          if progress < 90.0 {
            progress = (progress + 10.0).min(90.0);
            let _ = app.emit("task-progress", &serde_json::json!({
              "taskId": task_id_for_worker,
              "progress": progress
            }));
            let mut store = tasks.lock().await;
            if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
              job.progress = Some(progress);
              job.step = Some("running".to_string());
            }
          }
        }
      }
    };

    let (status, error) = match result {
      Ok(()) => (JobStatus::Succeeded, None),
      Err(err) => (JobStatus::Failed, Some(err.to_string())),
    };

    let mut store = tasks.lock().await;
    if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
      let step = if status == JobStatus::Succeeded {
        "completed"
      } else {
        "failed"
      };
      job.status = status.clone();
      job.finished_at = Some(Utc::now());
      job.error = error.clone();
      job.progress = Some(100.0);
      job.step = Some(step.to_string());
    }

    if let Some(logs) = store.logs.get_mut(&task_id_for_worker) {
      if let Some(error) = error {
        logs.push(error);
      } else {
        logs.push("Completed".to_string());
      }
    }

    store.handles.remove(&task_id_for_worker);
    drop(store);

    if let Err(err) = cache.invalidate(&cache_key).await {
      let mut store = tasks.lock().await;
      if let Some(logs) = store.logs.get_mut(&task_id_for_worker) {
        logs.push(format!("清除缓存失败: {}", err));
      }
    }

    let _ = app.emit("task-progress", &serde_json::json!({
      "taskId": task_id_for_worker,
      "progress": 100
    }));
    let _ = app.emit("task-complete", &serde_json::json!({
      "id": task_id_for_worker,
      "status": format!("{:?}", status),
      "manager": manager_for_worker
    }));
  });

  {
    let mut store = state.tasks.lock().await;
    store.handles.insert(task_id.clone(), task_handle);
  }

  Ok(task_id)
}

async fn spawn_batch_update(
  app: AppHandle,
  state: &AppState,
  manager: String,
  global: bool,
  workdir: Option<PathBuf>,
) -> Result<String, String> {
  let task_id = Uuid::new_v4().to_string();
  let job = Job {
    id: task_id.clone(),
    manager: manager.clone(),
    operation: Operation::Update,
    target: "outdated".to_string(),
    status: JobStatus::Running,
    progress: Some(0.0),
    step: Some("started".to_string()),
    started_at: Some(Utc::now()),
    finished_at: None,
    logs: vec!["开始批量更新过时包".to_string()],
    error: None,
  };

  {
    let mut store = state.tasks.lock().await;
    store.tasks.push(job.clone());
    store.logs.insert(task_id.clone(), Vec::new());
  }

  let _ = app.emit("task-progress", &serde_json::json!({
    "taskId": task_id,
    "progress": 0
  }));

  let cache = state.cache.clone();
  let tasks = state.tasks.clone();
  let executor = state.executor.clone();
  let task_id_for_worker = task_id.clone();
  let manager_for_worker = manager.clone();
  let task_handle = tokio::spawn(async move {
    let manager_impl = create_manager(&manager, cache.clone(), global, workdir.clone());
    let cache_key = manager_impl
      .as_ref()
      .map(|mgr| mgr.cache_key().to_string())
      .unwrap_or_else(|| manager.clone());

    let outdated = match manager_impl {
      Some(ref mgr) => match mgr.check_outdated().await {
        Ok(list) => list,
        Err(err) => {
          let mut store = tasks.lock().await;
          if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
            job.status = JobStatus::Failed;
            job.finished_at = Some(Utc::now());
            job.error = Some(err.to_string());
            job.progress = Some(100.0);
            job.step = Some("failed".to_string());
          }
          if let Some(logs) = store.logs.get_mut(&task_id_for_worker) {
            logs.push(format!("检查过时包失败: {}", err));
          }
          store.handles.remove(&task_id_for_worker);
          let _ = app.emit("task-complete", &serde_json::json!({
            "id": task_id_for_worker,
            "status": "Failed",
            "manager": manager_for_worker
          }));
          return;
        }
      },
      None => {
        let mut store = tasks.lock().await;
        if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
          job.status = JobStatus::Failed;
          job.finished_at = Some(Utc::now());
          job.error = Some("unknown manager".to_string());
          job.progress = Some(100.0);
          job.step = Some("failed".to_string());
        }
        if let Some(logs) = store.logs.get_mut(&task_id_for_worker) {
          logs.push("未知的包管理器".to_string());
        }
        store.handles.remove(&task_id_for_worker);
        let _ = app.emit("task-complete", &serde_json::json!({
          "id": task_id_for_worker,
          "status": "Failed",
          "manager": manager_for_worker
        }));
        return;
      }
    };

    if outdated.is_empty() {
      let mut store = tasks.lock().await;
      if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
        job.status = JobStatus::Succeeded;
        job.finished_at = Some(Utc::now());
        job.progress = Some(100.0);
        job.step = Some("completed".to_string());
      }
      if let Some(logs) = store.logs.get_mut(&task_id_for_worker) {
        logs.push("没有可更新的包".to_string());
      }
      store.handles.remove(&task_id_for_worker);
      let _ = app.emit("task-progress", &serde_json::json!({
        "taskId": task_id_for_worker,
        "progress": 100
      }));
      let _ = app.emit("task-complete", &serde_json::json!({
        "id": task_id_for_worker,
        "status": "Succeeded",
        "manager": manager_for_worker
      }));
      return;
    }

    let total = outdated.len().max(1) as f64;
    for (index, pkg) in outdated.iter().enumerate() {
      let result = executor
        .execute(&manager, || async {
          let manager_impl = create_manager(&manager, cache.clone(), global, workdir.clone())
            .ok_or_else(|| {
            boxy_error::BoxyError::ManagerNotFound {
              name: manager.clone(),
            }
          })?;
          manager_impl.upgrade(&pkg.name).await
        })
        .await;

      let progress = 10.0 + ((index as f64 + 1.0) / total * 80.0);
      let mut store = tasks.lock().await;
      if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
        job.progress = Some(progress.min(90.0));
        job.step = Some("running".to_string());
      }
      if let Some(logs) = store.logs.get_mut(&task_id_for_worker) {
        match &result {
          Ok(()) => logs.push(format!("更新 {} 成功", pkg.name)),
          Err(err) => logs.push(format!("更新 {} 失败: {}", pkg.name, err)),
        }
      }
      let _ = app.emit("task-progress", &serde_json::json!({
        "taskId": task_id_for_worker,
        "progress": progress.min(90.0)
      }));

      if result.is_err() {
        let mut store = tasks.lock().await;
        if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
          job.status = JobStatus::Failed;
          job.finished_at = Some(Utc::now());
          job.error = Some("批量更新失败".to_string());
          job.progress = Some(100.0);
          job.step = Some("failed".to_string());
        }
        store.handles.remove(&task_id_for_worker);
        let _ = app.emit("task-progress", &serde_json::json!({
          "taskId": task_id_for_worker,
          "progress": 100
        }));
        let _ = app.emit("task-complete", &serde_json::json!({
          "id": task_id_for_worker,
          "status": "Failed",
          "manager": manager_for_worker
        }));
        let _ = cache.invalidate(&cache_key).await;
        return;
      }
    }

    let mut store = tasks.lock().await;
    if let Some(job) = store.tasks.iter_mut().find(|job| job.id == task_id_for_worker) {
      job.status = JobStatus::Succeeded;
      job.finished_at = Some(Utc::now());
      job.progress = Some(100.0);
      job.step = Some("completed".to_string());
    }
    store.handles.remove(&task_id_for_worker);
    drop(store);

    let _ = cache.invalidate(&cache_key).await;

    let _ = app.emit("task-progress", &serde_json::json!({
      "taskId": task_id_for_worker,
      "progress": 100
    }));
    let _ = app.emit("task-complete", &serde_json::json!({
      "id": task_id_for_worker,
      "status": "Succeeded",
      "manager": manager_for_worker
    }));
  });

  {
    let mut store = state.tasks.lock().await;
    store.handles.insert(task_id.clone(), task_handle);
  }

  Ok(task_id)
}

impl TaskStore {
  pub fn new() -> Self {
    Self {
      tasks: Vec::new(),
      logs: HashMap::new(),
      handles: HashMap::new(),
    }
  }
}
