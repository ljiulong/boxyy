use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

pub struct UsageBar {
  text: String,
}

impl UsageBar {
  pub fn new(text: impl Into<String>) -> Self {
    Self { text: text.into() }
  }

  pub fn render(&self, area: Rect, buf: &mut Buffer) {
    let line = Line::from(vec![Span::styled(
      self.text.as_str(),
      Style::default().fg(Color::Gray),
    )]);

    let paragraph = Paragraph::new(line).block(Block::default().borders(Borders::TOP));
    paragraph.render(area, buf);
  }
}
