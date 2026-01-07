use crate::app::InputMode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct StatusBar {
  mode: InputMode,
  message: String,
}

impl StatusBar {
  pub fn new(mode: InputMode, message: impl Into<String>) -> Self {
    Self {
      mode,
      message: message.into(),
    }
  }

  pub fn render(&self, area: Rect, buf: &mut Buffer) {
    let mode_text = match self.mode {
      InputMode::Normal => "NORMAL",
      InputMode::Search => "SEARCH",
      InputMode::ActionMenu => "ACTION",
    };

    // 检查消息中是否包含更新提示
    let has_update_warning = self.message.contains("⚠️") || self.message.contains("可更新");
    let message_style = if has_update_warning {
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
      Style::default().fg(Color::White)
    };

    let spans = vec![
      Span::styled(
        format!("[{}] ", mode_text),
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
      ),
      Span::styled(self.message.as_str(), message_style),
    ];

    let paragraph = Paragraph::new(Line::from(spans))
      .block(
        Block::default()
          .borders(Borders::TOP)
          .style(if has_update_warning {
            Style::default().fg(Color::Yellow)
          } else {
            Style::default().fg(Color::White)
          }),
      );

    paragraph.render(area, buf);
  }
}
