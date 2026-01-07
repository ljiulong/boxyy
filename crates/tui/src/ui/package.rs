use crate::app::App;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};
use ratatui::Frame;

pub fn draw_package_detail(f: &mut Frame, app: &mut App, area: Rect) {
  let block = Block::default().borders(Borders::ALL).title("Package Details");
  let mut lines = Vec::new();

  if let Some(pkg) = app.selected_package() {
    lines.push(Line::from(vec![Span::styled(
      pkg.name.as_str(),
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )]));
    if !pkg.version.is_empty() {
      lines.push(Line::from(format!("Version: {}", pkg.version)));
    }
    lines.push(Line::from(format!("Manager: {}", pkg.manager)));

    if let Some(desc) = &pkg.description {
      lines.push(Line::from(""));
      lines.push(Line::from(desc.as_str()));
    }

    if let Some(homepage) = &pkg.homepage {
      lines.push(Line::from(""));
      lines.push(Line::from(format!("Homepage: {}", homepage)));
    }

    if let Some(license) = &pkg.license {
      lines.push(Line::from(format!("License: {}", license)));
    }

    if let Some(path) = &pkg.installed_path {
      lines.push(Line::from(format!("Path: {}", path)));
    }

    if let Some(size) = pkg.size {
      lines.push(Line::from(format!("Size: {} bytes", size)));
    }

    if let Some(latest) = &pkg.latest_version {
      lines.push(Line::from(format!("Latest: {}", latest)));
    }
  } else {
    lines.push(Line::from("No package selected"));
  }

  let paragraph = Paragraph::new(Text::from(lines)).block(block).wrap(Wrap { trim: true });
  paragraph.render(area, f.buffer_mut());
}
