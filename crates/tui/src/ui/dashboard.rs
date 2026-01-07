use crate::app::App;
use crate::components::list::ListWidget;
use crate::managers::supports_global;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};
use ratatui::Frame;

pub fn draw_dashboard(f: &mut Frame, app: &mut App, area: Rect) {
  let columns = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([
      Constraint::Percentage(25),
      Constraint::Percentage(50),
      Constraint::Percentage(25),
    ])
    .split(area);

  draw_manager_list(f, app, columns[0]);
  draw_package_list(f, app, columns[1]);
  draw_package_detail(f, app, columns[2]);
}

fn draw_manager_list(f: &mut Frame, app: &mut App, area: Rect) {
  let visible_height = area.height.saturating_sub(2) as usize;
  
  // 构建标题，显示当前模式
  let manager_name = app.selected_manager_name();
  let mode_text = if let Some(name) = manager_name {
    if supports_global(name) {
      if app.global {
        "Managers [🌐 Global]"
      } else {
        "Managers [📁 Local]"
      }
    } else {
      "Managers"
    }
  } else {
    "Managers"
  };
  
  let list = ListWidget::new(
    &app.managers,
    app.selected_manager_index,
    visible_height,
    mode_text,
    |item, _selected| {
      const STATUS_AVAILABLE: &str = "[+]";
      const STATUS_UNAVAILABLE: &str = "[x]";
      let status = if item.available { STATUS_AVAILABLE } else { STATUS_UNAVAILABLE };
      let status_color = if item.available { Color::Green } else { Color::Red };
      let name_style = if item.available {
        Style::default().add_modifier(Modifier::BOLD)
      } else {
        Style::default().fg(Color::DarkGray)
      };
      let count = format!("{}", item.package_count);
      let outdated = if item.outdated_count > 0 {
        format!(" !{}", item.outdated_count)
      } else {
        String::new()
      };

      Line::from(vec![
        Span::styled(status, Style::default().fg(status_color)),
        Span::raw(" "),
        Span::styled(item.name.as_str(), name_style),
        Span::raw(" ("),
        Span::styled(count, Style::default().fg(Color::Cyan)),
        Span::raw(")"),
        Span::styled(outdated, Style::default().fg(Color::Yellow)),
      ])
    },
  );

  list.render(area, f.buffer_mut());
}

fn draw_package_list(f: &mut Frame, app: &mut App, area: Rect) {
  let visible_height = area.height.saturating_sub(2) as usize;
  let list = ListWidget::new(
    &app.packages,
    app.selected_package_index,
    visible_height,
    "Packages",
    |item, selected| {
      let version = if item.version.is_empty() {
        "".to_string()
      } else {
        format!(" {}", item.version)
      };
      let outdated = if item.outdated { " !" } else { "" };

      // 如果有更新，使用黄色高亮包名；如果被选中，使用默认高亮
      let name_style = if selected {
        Style::default() // 选中时使用 ListWidget 的默认高亮
      } else if item.outdated {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
      } else {
        Style::default()
      };

      Line::from(vec![
        Span::styled(item.name.as_str(), name_style),
        Span::styled(version, Style::default().fg(Color::Gray)),
        Span::styled(outdated, Style::default().fg(Color::Yellow)),
      ])
    },
  );

  list.render(area, f.buffer_mut());
}

fn draw_package_detail(f: &mut Frame, app: &mut App, area: Rect) {
  let block = Block::default().borders(Borders::ALL).title("Details");
  let mut lines = Vec::new();

  if let Some(pkg) = app.selected_package() {
    lines.push(Line::from(vec![Span::styled(
      pkg.name.as_str(),
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]));
    if !pkg.version.is_empty() {
      lines.push(Line::from(format!("Version: {}", pkg.version)));
    }
    if let Some(desc) = &pkg.description {
      lines.push(Line::from(""));
      lines.push(Line::from(desc.as_str()));
    }
    // 显示更新信息
    if pkg.outdated {
      lines.push(Line::from(""));
      if let Some(latest) = &pkg.latest_version {
        // 有最新版本号，显示版本对比
        lines.push(Line::from(vec![
          Span::styled(
            "⚠️ 有更新可用 / Update available: ",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
          ),
          Span::styled(
            format!("{} → {}", pkg.version, latest),
            Style::default().fg(Color::Green),
          ),
        ]));
      } else {
        // 没有最新版本号，但标记为过时
        lines.push(Line::from(vec![Span::styled(
          "⚠️ 有更新可用 / Update available",
          Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]));
      }
    } else if let Some(latest) = &pkg.latest_version {
      // 没有更新，但显示最新版本号
      lines.push(Line::from(""));
      lines.push(Line::from(format!("Latest: {}", latest)));
    }
    
    // 显示当前任务进度（如果有）
    if let Some(job) = &app.current_job {
      if job.status == boxy_core::JobStatus::Running {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
          "任务进度 / Task Progress:",
          Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]));
        
        let progress_text = if let Some(progress) = job.progress {
          format!("{}%", progress as u32)
        } else {
          "进行中...".to_string()
        };
        
        let operation_text = match job.operation {
          boxy_core::Operation::Update => "更新",
          boxy_core::Operation::Uninstall => "卸载",
          boxy_core::Operation::Install => "安装",
        };
        
        lines.push(Line::from(vec![
          Span::styled(
            format!("  {} {}: ", operation_text, job.target),
            Style::default().fg(Color::Cyan),
          ),
          Span::styled(
            progress_text,
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
          ),
        ]));
        
        // 显示进度条
        if let Some(progress) = job.progress {
          let bar_width = 20;
          let filled = (progress / 100.0 * bar_width as f64) as usize;
          let bar = format!(
            "[{}{}]",
            "█".repeat(filled.min(bar_width)),
            "░".repeat(bar_width.saturating_sub(filled))
          );
          lines.push(Line::from(vec![
            Span::styled(
              format!("  {}", bar),
              Style::default().fg(Color::Green),
            ),
          ]));
        }
      }
    }
    
    // 显示操作选项
    lines.push(Line::from(""));
    
    // 如果处于操作菜单模式，显示更明显的提示
    if app.input_mode == crate::app::InputMode::ActionMenu {
      lines.push(Line::from(vec![Span::styled(
        "═══ 操作菜单 / Action Menu ═══",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
      )]));
      lines.push(Line::from(""));
    } else {
      lines.push(Line::from(vec![Span::styled(
        "操作 / Actions:",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
      )]));
      lines.push(Line::from(vec![Span::styled(
        "  (按 [a] 或 [Enter] 打开操作菜单)",
        Style::default().fg(Color::DarkGray),
      )]));
    }
    
    // 更新选项
    let update_text = if pkg.outdated {
      "  ▶ 更新到最新版本 / Update to latest"
    } else {
      "  ▶ 更新 / Update"
    };
    let update_style = if app.input_mode == crate::app::InputMode::ActionMenu
      && app.selected_action_index == 0
    {
      Style::default()
        .fg(Color::Black)
        .bg(Color::Green)
        .add_modifier(Modifier::BOLD)
    } else if app.input_mode == crate::app::InputMode::ActionMenu {
      Style::default().fg(Color::Green)
    } else {
      Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![Span::styled(update_text, update_style)]));
    
    // 卸载选项
    let uninstall_text = "  ▶ 卸载 / Uninstall";
    let uninstall_style = if app.input_mode == crate::app::InputMode::ActionMenu
      && app.selected_action_index == 1
    {
      Style::default()
        .fg(Color::Black)
        .bg(Color::Red)
        .add_modifier(Modifier::BOLD)
    } else if app.input_mode == crate::app::InputMode::ActionMenu {
      Style::default().fg(Color::Red)
    } else {
      Style::default().fg(Color::DarkGray)
    };
    lines.push(Line::from(vec![Span::styled(uninstall_text, uninstall_style)]));
    
    // 全局/本地切换提示（如果支持）
    if let Some(manager_name) = app.selected_manager_name() {
      if supports_global(manager_name) {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
          "模式切换 / Mode Toggle:",
          Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]));
        let mode_text = if app.global {
          "  [g] 切换到本地模式 / Switch to Local"
        } else {
          "  [g] 切换到全局模式 / Switch to Global"
        };
        lines.push(Line::from(vec![Span::styled(
          mode_text,
          Style::default().fg(Color::Cyan),
        )]));
      }
    }
    
    // 退出提示
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
      "退出 / Exit:",
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(vec![
      Span::styled("  [q] ", Style::default().fg(Color::Gray)),
      Span::styled("或 / or ", Style::default().fg(Color::DarkGray)),
      Span::styled("[Ctrl+C] ", Style::default().fg(Color::Gray)),
      Span::styled("退出应用 / Quit", Style::default().fg(Color::DarkGray)),
    ]));

    // 版本号显示
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
      format!("Boxy v{}", env!("CARGO_PKG_VERSION")),
      Style::default().fg(Color::DarkGray),
    )]));
  } else {
    lines.push(Line::from("No package selected"));

    // 版本号显示
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
      format!("Boxy v{}", env!("CARGO_PKG_VERSION")),
      Style::default().fg(Color::DarkGray),
    )]));
  }

  let paragraph = Paragraph::new(Text::from(lines))
    .block(block)
    .wrap(ratatui::widgets::Wrap { trim: true });
  paragraph.render(area, f.buffer_mut());
}
