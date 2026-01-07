use crate::app::App;
use anyhow::Result;
use boxy_cache::Cache;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::execute;
use crossterm::terminal::{
  disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
#[cfg(target_os = "macos")]
use std::collections::HashSet;
#[cfg(target_os = "macos")]
use std::env;
use std::io::{stdout, Stdout};
#[cfg(target_os = "macos")]
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::time::sleep;

mod app;
mod components;
mod managers;
mod ui;

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const INPUT_POLL_INTERVAL: Duration = Duration::from_millis(200);

struct TerminalGuard;

impl Drop for TerminalGuard {
  fn drop(&mut self) {
    let _ = disable_raw_mode();
    let mut stdout = stdout();
    let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
  }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
  enable_raw_mode()?;
  let mut stdout = stdout();
  execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
  let backend = CrosstermBackend::new(stdout);
  Ok(Terminal::new(backend)?)
}

#[tokio::main]
async fn main() -> Result<()> {
  ensure_macos_path();
  if std::env::args().skip(1).any(|arg| arg == "--version" || arg == "-V") {
    println!("boxy-tui {}", env!("CARGO_PKG_VERSION"));
    return Ok(());
  }

  let _guard = TerminalGuard;
  let mut terminal = setup_terminal()?;

  let cache = Arc::new(Cache::new()?);
  let app = App::new(cache).await?;
  let app = Arc::new(Mutex::new(app));
  
  // 启动后立即在后台加载当前管理器的包列表
  {
    let app_handle = app.clone();
    tokio::spawn(async move {
      let mut app = app_handle.lock().await;
      if let Err(err) = app.load_packages_for_selected_manager().await {
        app.status_message = format!("加载包列表失败: {}", err);
        app.should_redraw = true;
      }
    });
  }

  let refresh_handle = app.clone();
  let refresh_task = tokio::spawn(async move {
    loop {
      sleep(REFRESH_INTERVAL).await;
      let selected_manager = {
        let mut app = refresh_handle.lock().await;
        let _ = app.refresh_manager_availability().await;
        app.selected_manager_name().map(|name| name.to_string())
      };

      if let Some(manager) = selected_manager {
        let mut app = refresh_handle.lock().await;
        if app.selected_manager_name() == Some(manager.as_str()) {
          app.schedule_load_packages(refresh_handle.clone());
        }
      }
    }
  });

  loop {
    {
      let mut app = app.lock().await;
      if app.should_quit {
        break;
      }
      if app.should_redraw {
        terminal.draw(|f| ui::draw(f, &mut app))?;
        app.should_redraw = false;
      }
    }

    if event::poll(INPUT_POLL_INTERVAL)? {
      if let Event::Key(key) = event::read()? {
        let app_handle = app.clone();
        let mut app = app.lock().await;
        app.handle_key_event(key, app_handle).await;
      }
    }
  }

  refresh_task.abort();

  Ok(())
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
