use crate::managers::{create_manager, supports_global, MANAGER_NAMES};
use anyhow::{Context, Result};
use boxy_cache::Cache;
use boxy_core::{Job, JobStatus, ManagerExecutor, ManagerStatus, Operation, Package};
use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, timeout, Duration};

pub struct App {
  pub current_view: View,
  pub input_mode: InputMode,
  pub managers: Vec<ManagerStatus>,
  pub selected_manager_index: usize,
  pub packages: Vec<Package>,
  pub packages_all: Vec<Package>,
  pub selected_package_index: usize,
  pub selected_action_index: usize,
  pub global: bool,
  pub jobs: Vec<Job>,
  pub current_job: Option<Job>,
  pub search_query: String,
  pub cache: Arc<Cache>,
  pub executor: Arc<ManagerExecutor>,
  pub should_quit: bool,
  pub should_redraw: bool,
  pub status_message: String,
  pub show_help: bool,
  pub modal: Option<ModalState>,
  pub pending_action: Option<PendingAction>,
  job_handles: HashMap<String, tokio::task::JoinHandle<()>>,
  job_counter: u64,
  last_view: Option<View>,
  pub last_refresh: Option<DateTime<Utc>>,
  load_packages_request_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum View {
  Dashboard,
  ManagerDetail(String),
  PackageDetail(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
  Normal,
  Search,
  ActionMenu,
}

#[derive(Debug, Clone)]
pub enum PendingAction {
  Update { manager: String, package: String },
  Uninstall {
    manager: String,
    package: String,
    force: bool,
  },
}

#[derive(Debug, Clone)]
pub enum ModalState {
  Confirm { title: String, message: String },
  Logs { title: String, lines: Vec<String> },
  Success { title: String, message: String },
  Error { title: String, message: String },
}

impl App {
  pub async fn new(cache: Arc<Cache>) -> Result<Self> {
    let mut app = Self {
      current_view: View::Dashboard,
      input_mode: InputMode::Normal,
      managers: Vec::new(),
      selected_manager_index: 0,
      packages: Vec::new(),
      packages_all: Vec::new(),
      selected_package_index: 0,
      selected_action_index: 0,
      global: false,
      jobs: Vec::new(),
      current_job: None,
      search_query: String::new(),
      cache,
      executor: Arc::new(ManagerExecutor::default()),
      should_quit: false,
      should_redraw: true,
      status_message: "正在初始化...".to_string(),
      show_help: false,
      modal: None,
      pending_action: None,
      job_handles: HashMap::new(),
      job_counter: 0,
      last_view: None,
      last_refresh: None,
      load_packages_request_id: 0,
    };

    // 快速初始化：先显示 UI，然后在后台加载数据
    // 使用超时控制，避免阻塞太久（最多等待1秒）
    let refresh_result = timeout(Duration::from_secs(1), app.refresh_manager_availability()).await;
    match refresh_result {
      Ok(Ok(_)) => {
        app.status_message = "就绪".to_string();
      }
      Ok(Err(err)) => {
        app.status_message = format!("初始化警告: {}", err);
        // 即使出错，也尝试从缓存加载
        app.load_manager_status_from_cache().await;
      }
      Err(_) => {
        // 超时，使用缓存数据快速显示
        app.status_message = "正在后台加载...".to_string();
        // 尝试从缓存快速加载管理器状态
        app.load_manager_status_from_cache().await;
      }
    }

    // 包列表加载改为后台任务，不阻塞启动
    // 先显示空列表，后台加载

    Ok(app)
  }

  // 从缓存快速加载管理器状态（不检查可用性）
  async fn load_manager_status_from_cache(&mut self) {
    let cache = self.cache.clone();
    let global = self.global;
    let mut managers = Vec::new();

    for name in MANAGER_NAMES.iter() {
      let manager = create_manager(name, cache.clone(), global);
      if let Some(mgr) = manager {
        let cache_key = mgr.cache_key();
        let cached_packages: Vec<Package> =
          cache.get(cache_key).await.unwrap_or(None).unwrap_or_default();
        let outdated_count = cached_packages.iter().filter(|pkg| pkg.outdated).count();
        managers.push(ManagerStatus {
          name: name.to_string(),
          version: "".to_string(),
          available: true, // 假设可用，稍后检查
          package_count: cached_packages.len(),
          outdated_count,
        });
      } else {
        managers.push(ManagerStatus {
          name: name.to_string(),
          version: "".to_string(),
          available: false,
          package_count: 0,
          outdated_count: 0,
        });
      }
    }

    self.managers = managers;
    if self.selected_manager_index >= self.managers.len() {
      self.selected_manager_index = 0;
    }
  }

  pub fn selected_manager_name(&self) -> Option<&str> {
    self.managers
      .get(self.selected_manager_index)
      .map(|m| m.name.as_str())
  }

  pub fn selected_package(&self) -> Option<&Package> {
    self.packages.get(self.selected_package_index)
  }

  pub async fn refresh_manager_availability(&mut self) -> Result<()> {
    let cache = self.cache.clone();
    let global = self.global;
    let tasks: Vec<_> = MANAGER_NAMES
      .iter()
      .map(|name| {
        let cache = cache.clone();
        let manager_name = name.to_string();
        let global = global;
        tokio::spawn(async move {
          let manager = create_manager(&manager_name, cache.clone(), global);
          if let Some(mgr) = manager {
            // 为可用性检查添加超时（500ms），避免阻塞太久
            let available = timeout(Duration::from_millis(500), mgr.check_available())
              .await
              .unwrap_or(Ok(false))
              .unwrap_or(false);
            // 使用管理器的 cache_key 来获取正确的缓存数据
            let cache_key = mgr.cache_key();
            let cached_packages: Vec<Package> =
              cache.get(cache_key).await.unwrap_or(None).unwrap_or_default();
            let outdated_count = cached_packages.iter().filter(|pkg| pkg.outdated).count();
            ManagerStatus {
              name: manager_name,
              version: "".to_string(),
              available,
              package_count: cached_packages.len(),
              outdated_count,
            }
          } else {
            ManagerStatus {
              name: manager_name,
              version: "".to_string(),
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

    self.managers = managers;
    if self.selected_manager_index >= self.managers.len() {
      self.selected_manager_index = 0;
    }
    self.last_refresh = Some(Utc::now());
    self.status_message = format!(
      "管理器状态已刷新 ({})",
      if global { "全局模式" } else { "本地模式" }
    );
    self.should_redraw = true;

    Ok(())
  }

  pub async fn load_packages_for_selected_manager(&mut self) -> Result<()> {
    let manager_name = self.selected_manager_name().map(|s| s.to_string());
    let Some(manager_name) = manager_name else {
      self.packages = Vec::new();
      self.packages_all = Vec::new();
      return Ok(());
    };

    let (packages, outdated_map) =
      fetch_packages(&manager_name, self.cache.clone(), self.global).await?;
    let mut packages = packages;
    for pkg in packages.iter_mut() {
      if let Some(latest) = outdated_map.get(&pkg.name) {
        pkg.outdated = true;
        pkg.latest_version = latest.clone();
      }
    }
    self.packages_all = packages;
    self.apply_search_filter();
    self.selected_package_index = 0;
    self.status_message = format!("Loaded {} packages", self.packages.len());
    self.update_manager_counts(&manager_name);
    self.should_redraw = true;

    Ok(())
  }

  pub fn schedule_load_packages(&mut self, handle: Arc<Mutex<App>>) {
    self.load_packages_request_id = self.load_packages_request_id.wrapping_add(1);
    let request_id = self.load_packages_request_id;
    self.status_message = "Loading packages...".to_string();
    self.should_redraw = true;
    tokio::spawn(async move {
      let (manager_name, cache, global) = {
        let app = handle.lock().await;
        let Some(name) = app.selected_manager_name().map(|s| s.to_string()) else {
          return;
        };
        (name, app.cache.clone(), app.global)
      };

      let result = fetch_packages(&manager_name, cache, global).await;
      let mut app = handle.lock().await;
      if app.load_packages_request_id != request_id {
        return;
      }
      if app.selected_manager_name() != Some(manager_name.as_str()) {
        return;
      }
      match result {
        Ok((mut packages, outdated_map)) => {
          for pkg in packages.iter_mut() {
            if let Some(latest) = outdated_map.get(&pkg.name) {
              pkg.outdated = true;
              pkg.latest_version = latest.clone();
            }
          }
          app.packages_all = packages;
          app.apply_search_filter();
          app.selected_package_index = 0;
          app.status_message = format!("Loaded {} packages", app.packages.len());
          app.update_manager_counts(&manager_name);
          app.should_redraw = true;
        }
        Err(err) => {
          app.status_message = format!("Failed to load packages: {}", err);
          app.should_redraw = true;
        }
      }
    });
  }

  fn update_manager_counts(&mut self, manager_name: &str) {
    if let Some(manager) = self.managers.iter_mut().find(|m| m.name == manager_name) {
      manager.package_count = self.packages_all.len();
      manager.outdated_count = self
        .packages_all
        .iter()
        .filter(|pkg| pkg.outdated)
        .count();
    }
  }

  fn apply_search_filter(&mut self) {
    if self.search_query.is_empty() {
      self.packages = self.packages_all.clone();
      return;
    }

    let query = self.search_query.to_lowercase();
    self.packages = self
      .packages_all
      .iter()
      .filter(|pkg| pkg.name.to_lowercase().contains(&query))
      .cloned()
      .collect();

    if self.selected_package_index >= self.packages.len() {
      self.selected_package_index = 0;
    }
  }

  pub fn select_next_package(&mut self) {
    if self.packages.is_empty() {
      return;
    }
    self.selected_package_index = (self.selected_package_index + 1) % self.packages.len();
  }

  pub fn select_previous_package(&mut self) {
    if self.packages.is_empty() {
      return;
    }
    if self.selected_package_index == 0 {
      self.selected_package_index = self.packages.len() - 1;
    } else {
      self.selected_package_index -= 1;
    }
  }

  pub async fn select_next_manager(&mut self) -> Result<()> {
    if self.managers.is_empty() {
      return Ok(());
    }
    self.selected_manager_index = (self.selected_manager_index + 1) % self.managers.len();
    Ok(())
  }

  pub async fn select_previous_manager(&mut self) -> Result<()> {
    if self.managers.is_empty() {
      return Ok(());
    }
    if self.selected_manager_index == 0 {
      self.selected_manager_index = self.managers.len() - 1;
    } else {
      self.selected_manager_index -= 1;
    }
    Ok(())
  }

  pub fn enter_search_mode(&mut self) {
    self.input_mode = InputMode::Search;
    self.search_query.clear();
    self.status_message = "Search: type to filter, Enter to apply".to_string();
  }

  pub fn exit_search_mode(&mut self, clear: bool) {
    self.input_mode = InputMode::Normal;
    if clear {
      self.search_query.clear();
    }
    self.apply_search_filter();
    self.status_message = "Search exited".to_string();
  }

  pub fn enter_action_menu(&mut self) {
    if self.selected_package().is_some() {
      self.input_mode = InputMode::ActionMenu;
      self.selected_action_index = 0;
      self.status_message = "选择操作: j/k 移动, Enter 执行, Esc 取消".to_string();
      self.should_redraw = true;
    }
  }

  pub fn exit_action_menu(&mut self) {
    self.input_mode = InputMode::Normal;
    self.selected_action_index = 0;
    self.status_message = "操作菜单已退出".to_string();
    self.should_redraw = true;
  }

  pub fn select_next_action(&mut self) {
    // 操作选项：更新、卸载
    let action_count = 2;
    if action_count > 0 {
      self.selected_action_index = (self.selected_action_index + 1) % action_count;
      self.should_redraw = true;
    }
  }

  pub fn select_previous_action(&mut self) {
    let action_count = 2;
    if action_count > 0 {
      if self.selected_action_index == 0 {
        self.selected_action_index = action_count - 1;
      } else {
        self.selected_action_index -= 1;
      }
      self.should_redraw = true;
    }
  }

  pub fn execute_selected_action(&mut self, _handle: Arc<Mutex<App>>) {
    let manager = self.selected_manager_name().map(|s| s.to_string());
    let pkg = self.selected_package().map(|p| p.name.clone());
    let (Some(_manager), Some(_package)) = (manager, pkg) else {
      self.exit_action_menu();
      return;
    };

    match self.selected_action_index {
      0 => {
        // 更新
        self.exit_action_menu();
        self.request_update_selected();
      }
      1 => {
        // 卸载
        self.exit_action_menu();
        self.request_uninstall_selected(false);
      }
      _ => {
        self.exit_action_menu();
      }
    }
  }

  pub fn open_package_detail(&mut self) {
    let pkg_name = self.selected_package().map(|p| p.name.clone());
    if let Some(name) = pkg_name {
      self.last_view = Some(self.current_view.clone());
      self.current_view = View::PackageDetail(name);
      self.should_redraw = true;
    }
  }

  pub fn open_manager_detail(&mut self) {
    let manager_name = self.selected_manager_name().map(|s| s.to_string());
    if let Some(manager) = manager_name {
      self.last_view = Some(self.current_view.clone());
      self.current_view = View::ManagerDetail(manager);
      self.should_redraw = true;
    }
  }

  pub fn close_detail_view(&mut self) {
    if let Some(view) = self.last_view.take() {
      self.current_view = view;
    } else {
      self.current_view = View::Dashboard;
    }
    self.should_redraw = true;
  }

  pub fn toggle_help(&mut self) {
    self.show_help = !self.show_help;
    self.should_redraw = true;
  }

  pub fn toggle_global(&mut self) -> bool {
    // 检查当前选中的管理器是否支持全局模式
    if let Some(manager_name) = self.selected_manager_name() {
      if supports_global(manager_name) {
        self.global = !self.global;
        self.status_message = format!(
          "切换到 {} 模式",
          if self.global { "全局" } else { "本地" }
        );
        self.should_redraw = true;
        return true;
      }
    }
    false
  }

  pub fn show_logs_modal(&mut self) {
    if let Some(job) = self.current_job.as_ref() {
      self.modal = Some(ModalState::Logs {
        title: format!("Job {}", job.id),
        lines: job.logs.clone(),
      });
      self.should_redraw = true;
    }
  }

  pub fn close_modal(&mut self) {
    self.modal = None;
    self.pending_action = None;
    self.should_redraw = true;
  }

  pub async fn handle_key_event(&mut self, key: KeyEvent, handle: Arc<Mutex<App>>) {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
      self.should_quit = true;
      self.should_redraw = true;
      return;
    }

    if key.code == KeyCode::Char('q') {
      self.should_quit = true;
      self.should_redraw = true;
      return;
    }

    if key.code == KeyCode::Char('?') {
      self.toggle_help();
      return;
    }

    if self.modal.is_some() {
      self.handle_modal_key(key, handle).await;
      return;
    }

    match self.input_mode {
      InputMode::Normal => match self.current_view {
        View::Dashboard => self.handle_dashboard_keys(key, handle).await,
        View::ManagerDetail(_) => self.handle_manager_keys(key, handle).await,
        View::PackageDetail(_) => self.handle_package_keys(key).await,
      },
      InputMode::Search => self.handle_search_keys(key),
      InputMode::ActionMenu => self.handle_action_menu_keys(key, handle).await,
    }
    self.should_redraw = true;
  }

  async fn handle_dashboard_keys(&mut self, key: KeyEvent, handle: Arc<Mutex<App>>) {
    match key.code {
      KeyCode::Char('j') | KeyCode::Down => self.select_next_package(),
      KeyCode::Char('k') | KeyCode::Up => self.select_previous_package(),
      KeyCode::Char('h') | KeyCode::Left => {
        let _ = self.select_previous_manager().await;
        self.schedule_load_packages(handle);
      }
      KeyCode::Char('l') | KeyCode::Right => {
        let _ = self.select_next_manager().await;
        self.schedule_load_packages(handle);
      }
      KeyCode::Char('/') => self.enter_search_mode(),
      KeyCode::Char('a') => {
        if self.selected_package().is_some() {
          self.enter_action_menu();
        }
      },
      KeyCode::Char('u') => self.request_update_selected(),
      KeyCode::Char('d') => self.request_uninstall_selected(false),
      KeyCode::Char('c') => self.cancel_current_job(),
      KeyCode::Char('r') => {
        let _ = self.refresh_manager_availability().await;
        self.schedule_load_packages(handle);
      }
      KeyCode::Char('g') => {
        if self.toggle_global() {
          // 切换成功，刷新所有管理器的统计数据，然后重新加载当前管理器的包列表
          let handle_for_refresh = handle.clone();
          tokio::spawn(async move {
            {
              let mut app = handle_for_refresh.lock().await;
              let _ = app.refresh_manager_availability().await;
            }
            // 释放锁后再调用 schedule_load_packages
            let mut app = handle_for_refresh.lock().await;
            app.schedule_load_packages(handle_for_refresh.clone());
          });
        }
      },
      KeyCode::Enter => {
        if self.selected_package().is_some() {
          self.enter_action_menu();
        } else {
          self.open_package_detail();
        }
      },
      KeyCode::Char('m') => self.open_manager_detail(),
      KeyCode::Char('L') => {
        self.show_logs_modal();
      }
      _ => {}
    }
  }

  async fn handle_manager_keys(&mut self, key: KeyEvent, handle: Arc<Mutex<App>>) {
    match key.code {
      KeyCode::Char('j') | KeyCode::Down => self.select_next_package(),
      KeyCode::Char('k') | KeyCode::Up => self.select_previous_package(),
      KeyCode::Char('/') => self.enter_search_mode(),
      KeyCode::Char('a') | KeyCode::Enter => {
        if self.selected_package().is_some() {
          self.enter_action_menu();
        } else {
          self.open_package_detail();
        }
      },
      KeyCode::Char('u') => self.request_update_selected(),
      KeyCode::Char('d') => self.request_uninstall_selected(false),
      KeyCode::Char('c') => self.cancel_current_job(),
      KeyCode::Char('b') | KeyCode::Esc => self.close_detail_view(),
      KeyCode::Char('r') => {
        self.schedule_load_packages(handle);
      }
      KeyCode::Char('g') => {
        if self.toggle_global() {
          // 切换成功，刷新所有管理器的统计数据，然后重新加载当前管理器的包列表
          let handle_for_refresh = handle.clone();
          tokio::spawn(async move {
            {
              let mut app = handle_for_refresh.lock().await;
              let _ = app.refresh_manager_availability().await;
            }
            // 释放锁后再调用 schedule_load_packages
            let mut app = handle_for_refresh.lock().await;
            app.schedule_load_packages(handle_for_refresh.clone());
          });
        }
      },
      _ => {}
    }
  }

  async fn handle_package_keys(&mut self, key: KeyEvent) {
    match key.code {
      KeyCode::Char('b') | KeyCode::Esc => self.close_detail_view(),
      _ => {}
    }
  }

  fn handle_search_keys(&mut self, key: KeyEvent) {
    match key.code {
      KeyCode::Esc => self.exit_search_mode(true),
      KeyCode::Enter => self.exit_search_mode(false),
      KeyCode::Backspace => {
        self.search_query.pop();
        self.apply_search_filter();
      }
      KeyCode::Char(c) => {
        if !key.modifiers.contains(KeyModifiers::CONTROL) {
          self.search_query.push(c);
          self.apply_search_filter();
        }
      }
      _ => {}
    }
    self.should_redraw = true;
  }

  async fn handle_action_menu_keys(&mut self, key: KeyEvent, handle: Arc<Mutex<App>>) {
    match key.code {
      KeyCode::Esc => self.exit_action_menu(),
      KeyCode::Char('j') | KeyCode::Down => self.select_next_action(),
      KeyCode::Char('k') | KeyCode::Up => self.select_previous_action(),
      KeyCode::Enter => self.execute_selected_action(handle),
      _ => {}
    }
    self.should_redraw = true;
  }

  async fn handle_modal_key(&mut self, key: KeyEvent, handle: Arc<Mutex<App>>) {
    match self.modal.as_ref() {
      Some(ModalState::Confirm { .. }) => {
        match key.code {
          KeyCode::Char('y') | KeyCode::Enter => {
            if let Some(action) = self.pending_action.clone() {
              self.close_modal();
              let _ = self.perform_action(action, handle).await;
            } else {
              self.close_modal();
            }
          }
          KeyCode::Char('n') | KeyCode::Esc => {
            self.close_modal();
          }
          _ => {}
        }
      }
      Some(ModalState::Success { .. }) | Some(ModalState::Error { .. }) => {
        // 成功/失败提示：按任意键关闭
        self.close_modal();
      }
      Some(ModalState::Logs { .. }) => {
        // 日志查看：按 Esc 关闭
        if key.code == KeyCode::Esc {
          self.close_modal();
        }
      }
      None => {}
    }
    self.should_redraw = true;
  }

  fn request_update_selected(&mut self) {
    let manager = self.selected_manager_name().map(|s| s.to_string());
    let pkg = self.selected_package().map(|p| p.name.clone());
    let (Some(manager), Some(package)) = (manager, pkg) else {
      return;
    };

    let message = format!("Update {} from {}?", package, manager);
    self.pending_action = Some(PendingAction::Update {
      manager: manager.clone(),
      package: package.clone(),
    });
    self.modal = Some(ModalState::Confirm {
      title: "Update Package".to_string(),
      message,
    });
    self.should_redraw = true;
  }

  fn request_uninstall_selected(&mut self, force: bool) {
    let manager = self.selected_manager_name().map(|s| s.to_string());
    let pkg = self.selected_package().map(|p| p.name.clone());
    let (Some(manager), Some(package)) = (manager, pkg) else {
      return;
    };

    let message = format!("Uninstall {} from {}?", package, manager);
    self.pending_action = Some(PendingAction::Uninstall {
      manager: manager.clone(),
      package: package.clone(),
      force,
    });
    self.modal = Some(ModalState::Confirm {
      title: "Uninstall Package".to_string(),
      message,
    });
    self.should_redraw = true;
  }

  async fn perform_action(&mut self, action: PendingAction, handle: Arc<Mutex<App>>) -> Result<()> {
    match action {
      PendingAction::Update { manager, package } => {
        self.spawn_job(handle, manager, Operation::Update, package, false)
          .await;
      }
      PendingAction::Uninstall {
        manager,
        package,
        force,
      } => {
        self
          .spawn_job(handle, manager, Operation::Uninstall, package, force)
          .await;
      }
    }

    Ok(())
  }

  async fn spawn_job(
    &mut self,
    handle: Arc<Mutex<App>>,
    manager: String,
    operation: Operation,
    target: String,
    force: bool,
  ) {
    self.job_counter += 1;
    let job_id = format!("job-{}", self.job_counter);

    let job = Job {
      id: job_id.clone(),
      manager: manager.clone(),
      operation: operation.clone(),
      target: target.clone(),
      status: JobStatus::Running,
      progress: Some(0.0),
      step: Some("running".to_string()),
      started_at: Some(Utc::now()),
      finished_at: None,
      logs: vec![format!("Started {operation:?} {target}")],
      error: None,
    };

    self.current_job = Some(job.clone());
    self.jobs.push(job);
    self.status_message = format!("Job {} running", job_id);
    self.should_redraw = true;

    let cache = self.cache.clone();
    let executor = self.executor.clone();
    let global = self.global;
    let job_id_for_task = job_id.clone();
    let task = tokio::spawn(async move {
      let cache_key = create_manager(&manager, cache.clone(), global)
        .map(|mgr| mgr.cache_key().to_string())
        .unwrap_or_else(|| manager.clone());
      let mut progress: f64 = 0.0;
      let mut ticker = interval(Duration::from_secs(1));

      let operation_future = executor.execute(&manager, || async {
        let manager_impl = create_manager(&manager, cache.clone(), global);
        if let Some(mgr) = manager_impl {
          match operation {
            Operation::Update => mgr.upgrade(&target).await.map(|_| ()),
            Operation::Uninstall => mgr.uninstall(&target, force).await.map(|_| ()),
            Operation::Install => Ok(()),
          }
        } else {
          Err(boxy_error::BoxyError::ManagerNotFound {
            name: manager.clone(),
          })
        }
      });

      tokio::pin!(operation_future);
      let result = loop {
        tokio::select! {
          output = &mut operation_future => break output,
          _ = ticker.tick() => {
            if progress < 90.0 {
              progress = (progress + 10.0).min(90.0);
              let mut app = handle.lock().await;
              if let Some(job) = app.jobs.iter_mut().find(|job| job.id == job_id_for_task) {
                job.progress = Some(progress);
                job.step = Some("running".to_string());
              }
              if let Some(current) = app.current_job.as_mut() {
                if current.id == job_id_for_task {
                  current.progress = Some(progress);
                  current.step = Some("running".to_string());
                }
              }
              app.status_message =
                format!("Job {} running ({}%)", job_id_for_task, progress as u32);
              app.should_redraw = true;
            }
          }
        }
      };

      let (status, error_msg) = match result {
        Ok(()) => (JobStatus::Succeeded, None),
        Err(err) => (JobStatus::Failed, Some(err.to_string())),
      };

      let status_for_completion = status.clone();

      let mut app = handle.lock().await;
      if let Some(job) = app.jobs.iter_mut().find(|job| job.id == job_id_for_task) {
        let step = if status == JobStatus::Succeeded {
          "completed"
        } else {
          "failed"
        };
        job.status = status.clone();
        job.finished_at = Some(Utc::now());
        job.progress = Some(100.0);
        job.step = Some(step.to_string());
        if let Some(error) = error_msg.clone() {
          job.error = Some(error.clone());
          job.logs.push(error);
        } else {
          job.logs.push("Completed".to_string());
        }
      }

      if let Some(current) = app.current_job.as_mut() {
        if current.id == job_id_for_task {
          let step = if status == JobStatus::Succeeded {
            "completed"
          } else {
            "failed"
          };
          current.status = status.clone();
          current.progress = Some(100.0);
          current.step = Some(step.to_string());
        }
      }

      // 显示任务完成提示
      let completion_message = if status_for_completion == JobStatus::Succeeded {
        format!("{} 已成功{}", target, match operation {
          Operation::Update => "更新",
          Operation::Uninstall => "卸载",
          Operation::Install => "安装",
        })
      } else {
        format!("{} {}失败", target, match operation {
          Operation::Update => "更新",
          Operation::Uninstall => "卸载",
          Operation::Install => "安装",
        })
      };
      
      app.status_message = format!("Job {} finished", job_id_for_task);
      app.job_handles.remove(&job_id_for_task);
      
      // 显示完成提示 modal
      if status == JobStatus::Succeeded {
        app.modal = Some(ModalState::Success {
          title: "任务完成 / Task Completed".to_string(),
          message: completion_message,
        });
      } else {
        app.modal = Some(ModalState::Error {
          title: "任务失败 / Task Failed".to_string(),
          message: format!("{}\n错误: {}", completion_message, error_msg.as_deref().unwrap_or("未知错误")),
        });
      }
      
      // 任务完成后刷新包列表
      let handle_for_refresh = handle.clone();
      tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let mut app = handle_for_refresh.lock().await;
        app.schedule_load_packages(handle_for_refresh.clone());
      });
      
      app.should_redraw = true;
      drop(app);

      if let Err(err) = cache.invalidate(&cache_key).await {
        let mut app = handle.lock().await;
        if let Some(job) = app.jobs.iter_mut().find(|job| job.id == job_id_for_task) {
          job.logs.push(format!("清除缓存失败: {}", err));
        }
        app.status_message = format!("清除 {} 缓存失败", manager);
        app.should_redraw = true;
      }
    });
    self.job_handles.insert(job_id.clone(), task);
  }

  fn cancel_current_job(&mut self) {
    let job_id = self.current_job.as_ref().map(|job| job.id.clone());
    if let Some(job_id) = job_id {
      self.cancel_job(&job_id);
    }
  }

  fn cancel_job(&mut self, job_id: &str) {
    if let Some(handle) = self.job_handles.remove(job_id) {
      handle.abort();
    }

    if let Some(job) = self.jobs.iter_mut().find(|job| job.id == job_id) {
      job.status = JobStatus::Canceled;
      job.finished_at = Some(Utc::now());
      job.progress = Some(100.0);
      job.step = Some("canceled".to_string());
      job.logs.push("Canceled".to_string());
    }

    if let Some(current) = self.current_job.as_mut() {
      if current.id == job_id {
        current.status = JobStatus::Canceled;
      }
    }

    self.status_message = format!("Job {} canceled", job_id);
    self.should_redraw = true;
  }
}

async fn fetch_packages(
  manager_name: &str,
  cache: Arc<Cache>,
  global: bool,
) -> Result<(Vec<Package>, HashMap<String, Option<String>>)> {
  let manager = create_manager(manager_name, cache.clone(), global)
    .with_context(|| format!("unknown manager: {}", manager_name))?;

  if !manager.check_available().await.unwrap_or(false) {
    return Ok((Vec::new(), HashMap::new()));
  }

  let packages = manager.list_installed().await?;
  let outdated = match timeout(Duration::from_secs(5), manager.check_outdated()).await {
    Ok(Ok(list)) => list,
    Ok(Err(err)) => return Err(err.into()),
    Err(_) => return Err(anyhow::anyhow!("检查过时包超时")),
  };

  let mut outdated_map = HashMap::new();
  for pkg in outdated {
    outdated_map.insert(pkg.name, pkg.latest_version);
  }

  Ok((packages, outdated_map))
}
