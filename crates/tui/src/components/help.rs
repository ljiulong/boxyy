use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

pub fn render_help(f: &mut Frame, area: Rect) {
  let text = Text::from(vec![
    Line::from(vec![Span::styled(
      "使用指南 / Help",
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]),
    Line::from(""),
    Line::from(vec![Span::styled(
      "导航 / Navigation",
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]),
    Line::from("  j/k 或 ↑/↓    移动选择 / Move selection"),
    Line::from("  h/l 或 ←/→    切换包管理器 / Switch manager"),
    Line::from("  Enter         打开详情或操作菜单 / Open detail or action menu"),
    Line::from("  a             打开操作菜单 / Open action menu"),
    Line::from("  g             切换全局/本地模式 / Toggle global/local mode"),
    Line::from("                (仅支持 npm/pnpm/yarn/bun)"),
    Line::from(""),
    Line::from(vec![Span::styled(
      "搜索 / Search",
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]),
    Line::from("  /             进入搜索模式 / Enter search mode"),
    Line::from("  Esc           退出搜索 / Exit search"),
    Line::from(""),
    Line::from(vec![Span::styled(
      "包操作 / Package Actions",
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]),
    Line::from("  a/Enter       打开操作菜单 / Open action menu"),
    Line::from("  在操作菜单中: / In action menu:"),
    Line::from("    j/k 或 ↑/↓  选择操作 / Select action"),
    Line::from("    Enter       执行选中操作 / Execute selected action"),
    Line::from("    Esc         取消菜单 / Cancel menu"),
    Line::from("  u             更新选中的包 / Update selected package"),
    Line::from("  d             卸载选中的包 / Uninstall selected package"),
    Line::from(""),
    Line::from(vec![Span::styled(
      "任务管理 / Job Management",
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]),
    Line::from("  c             取消当前任务 / Cancel current job"),
    Line::from("  L             显示任务日志 / Show job logs"),
    Line::from(""),
    Line::from(vec![Span::styled(
      "导航与退出 / Navigation & Exit",
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]),
    Line::from("  b/Esc         退回上一级 / Go back"),
    Line::from("  r             刷新 / Refresh"),
    Line::from("  ?             切换帮助 / Toggle help"),
    Line::from("  q 或 Ctrl+C   退出应用 / Quit application"),
  ]);

  let block = Block::default()
    .title("帮助 / Help")
    .borders(Borders::ALL)
    .style(Style::default().fg(Color::White).bg(Color::Black));

  let paragraph = Paragraph::new(text).block(block).alignment(Alignment::Left);

  f.render_widget(Clear, area);
  f.render_widget(paragraph, area);
}
