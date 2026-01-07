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
use std::io::{stdout, Stdout};
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
