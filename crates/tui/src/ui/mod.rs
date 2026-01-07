use crate::app::{App, View};
use crate::components::help::render_help;
use crate::components::status_bar::StatusBar;
use crate::components::usage_bar::UsageBar;
use crate::managers::supports_global;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;

pub mod dashboard;
pub mod manager;
pub mod modal;
pub mod package;

pub fn draw(f: &mut Frame, app: &mut App) {
  let size = f.size();
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Min(1),
      Constraint::Length(1),
      Constraint::Length(1),
    ])
    .split(size);

  let content_area = chunks[0];
  let usage_area = chunks[1];
  let status_area = chunks[2];

  match app.current_view {
    View::Dashboard => dashboard::draw_dashboard(f, app, content_area),
    View::ManagerDetail(_) => manager::draw_manager_detail(f, app, content_area),
    View::PackageDetail(_) => package::draw_package_detail(f, app, content_area),
  }

  // 根据当前状态显示不同的操作提示
  let manager_name = app.selected_manager_name();
  let supports_global_mode = manager_name.map(|n| supports_global(n)).unwrap_or(false);
  
  // 构建全局/本地切换提示，确保始终显示（即使不支持也显示，但提示不可用）
  let global_hint = if supports_global_mode {
    format!("  [g] 切换{}", if app.global { "本地" } else { "全局" })
  } else {
    // 即使不支持，也显示提示，让用户知道这个功能存在
    "  [g] 切换模式(仅npm/pnpm/yarn/bun)".to_string()
  };
  
  let usage_text = match app.input_mode {
    crate::app::InputMode::ActionMenu => {
      "[j/k] 选择操作  [Enter] 执行  [Esc] 取消菜单  [q/Ctrl+C] 退出应用".to_string()
    }
    _ => {
      match app.current_view {
        crate::app::View::Dashboard => {
          let base = if app.selected_package().is_some() {
            "[a/Enter] 操作菜单  [j/k] 移动  [h/l] 切换管理器  [/] 搜索  [r] 刷新  [c] 取消"
          } else {
            "[j/k] 移动  [h/l] 切换管理器  [/] 搜索  [r] 刷新  [c] 取消"
          };
          format!("{}{}  [?] 帮助  [q/Ctrl+C] 退出应用", base, global_hint)
        }
        crate::app::View::ManagerDetail(_) => {
          let base = if app.selected_package().is_some() {
            "[a/Enter] 操作菜单  [j/k] 移动  [/] 搜索  [r] 刷新  [c] 取消"
          } else {
            "[j/k] 移动  [/] 搜索  [r] 刷新  [c] 取消"
          };
          format!("{}{}  [b/Esc] 退回上一级  [q/Ctrl+C] 退出应用", base, global_hint)
        }
        crate::app::View::PackageDetail(_) => {
          "[b/Esc] 退回上一级  [q/Ctrl+C] 退出应用".to_string()
        }
      }
    }
  };
  let usage = UsageBar::new(usage_text);
  usage.render(usage_area, f.buffer_mut());

  // 构建状态消息，包含更新提示和模式信息
  let mut status_message = app.status_message.clone();
  
  // 添加模式信息
  if let Some(manager_name) = app.selected_manager_name() {
    if supports_global(manager_name) {
      let mode = if app.global { "全局" } else { "本地" };
      status_message = format!("[{}] {}", mode, status_message);
    }
  }
  
  // 检查是否有可更新的包
  let outdated_count: usize = app.managers.iter().map(|m| m.outdated_count).sum();
  if outdated_count > 0 {
    let selected_outdated = app
      .packages_all
      .iter()
      .filter(|p| p.outdated)
      .count();
    
    if let Some(pkg) = app.selected_package() {
      if pkg.outdated {
        if let Some(latest) = &pkg.latest_version {
          status_message = format!(
            "{} | ⚠️  有更新可用: {} → {} | 当前管理器: {} 个包可更新",
            status_message,
            pkg.version,
            latest,
            selected_outdated
          );
        } else {
          status_message = format!(
            "{} | ⚠️  有更新可用 | 当前管理器: {} 个包可更新",
            status_message,
            selected_outdated
          );
        }
      } else if selected_outdated > 0 {
        status_message = format!(
          "{} | ⚠️  当前管理器有 {} 个包可更新",
          status_message,
          selected_outdated
        );
      }
    } else if selected_outdated > 0 {
      status_message = format!(
        "{} | ⚠️  当前管理器有 {} 个包可更新",
        status_message,
        selected_outdated
      );
    }
    
    if outdated_count > selected_outdated {
      status_message = format!(
        "{} | 总计: {} 个包可更新",
        status_message,
        outdated_count
      );
    }
  }

  let status = StatusBar::new(app.input_mode, status_message);
  status.render(status_area, f.buffer_mut());

  if app.show_help {
    let help_area = centered_rect(60, 60, size);
    render_help(f, help_area);
  }

  if let Some(modal_state) = app.modal.clone() {
    let modal_area = centered_rect(70, 40, size);
    modal::render_modal(f, modal_area, &modal_state);
  }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
  let popup_layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
      Constraint::Percentage((100 - percent_y) / 2),
      Constraint::Percentage(percent_y),
      Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

  Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Percentage((100 - percent_x) / 2),
      Constraint::Percentage(percent_x),
      Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
