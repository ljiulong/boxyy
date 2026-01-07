use crate::app::ModalState;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

pub fn render_modal(f: &mut Frame, area: Rect, modal: &ModalState) {
  let (title, body, title_color) = match modal {
    ModalState::Confirm { title, message } => {
      let lines = vec![
        Line::from(message.as_str()),
        Line::from(""),
        Line::from("按 Enter 确认，Esc 取消 / Press Enter to confirm, Esc to cancel"),
      ];
      (title.clone(), Text::from(lines), Color::Yellow)
    }
    ModalState::Logs { title, lines } => {
      let mut body_lines = Vec::new();
      for line in lines.iter().rev().take(20).rev() {
        body_lines.push(Line::from(line.as_str()));
      }
      if body_lines.is_empty() {
        body_lines.push(Line::from("No logs available"));
      }
      (title.clone(), Text::from(body_lines), Color::Yellow)
    }
    ModalState::Success { title, message } => {
      let lines = vec![
        Line::from(vec![Span::styled(
          "✓",
          Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(message.as_str()),
        Line::from(""),
        Line::from("按任意键关闭 / Press any key to close"),
      ];
      (title.clone(), Text::from(lines), Color::Green)
    }
    ModalState::Error { title, message } => {
      let lines: Vec<Line> = message
        .split('\n')
        .map(|s| Line::from(s))
        .collect();
      let mut body_lines = vec![
        Line::from(vec![Span::styled(
          "✗",
          Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
      ];
      body_lines.extend(lines);
      body_lines.push(Line::from(""));
      body_lines.push(Line::from("按任意键关闭 / Press any key to close"));
      (title.clone(), Text::from(body_lines), Color::Red)
    }
  };

  let block = Block::default()
    .title(Span::styled(
      title,
      Style::default().fg(title_color).add_modifier(Modifier::BOLD),
    ))
    .borders(Borders::ALL)
    .style(Style::default().fg(Color::White).bg(Color::Black));

  let paragraph = Paragraph::new(body)
    .block(block)
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: true });

  f.render_widget(Clear, area);
  f.render_widget(paragraph, area);
}
