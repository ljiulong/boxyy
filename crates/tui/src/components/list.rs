use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget};

pub struct ListWidget<'a, T> {
  items: &'a [T],
  selected: usize,
  visible_height: usize,
  render_item: Box<dyn Fn(&T, bool) -> Line + Send + Sync>,
  title: String,
}

impl<'a, T> ListWidget<'a, T> {
  pub fn new(
    items: &'a [T],
    selected: usize,
    visible_height: usize,
    title: impl Into<String>,
    render_item: impl Fn(&T, bool) -> Line + Send + Sync + 'static,
  ) -> Self {
    Self {
      items,
      selected,
      visible_height,
      render_item: Box::new(render_item),
      title: title.into(),
    }
  }

  pub fn render(&self, area: Rect, buf: &mut Buffer) {
    if area.height < 3 || area.width < 10 {
      return;
    }
    let visible_height = self.visible_height.max(1);
    let offset = if self.selected + 1 > visible_height {
      self.selected + 1 - visible_height
    } else {
      0
    };

    let end = (offset + visible_height).min(self.items.len());
    let visible_items = self.items.iter().skip(offset).take(end - offset);

    let items: Vec<ListItem> = visible_items
      .enumerate()
      .map(|(index, item)| {
        let selected = offset + index == self.selected;
        let line = (self.render_item)(item, selected);
        ListItem::new(line)
      })
      .collect();

    let block = Block::default().borders(Borders::ALL).title(self.title.as_str());
    let list = List::new(items)
      .block(block)
      .highlight_style(
        Style::default()
          .fg(Color::Black)
          .bg(Color::Cyan)
          .add_modifier(Modifier::BOLD),
      )
      .highlight_symbol("> ");

    let mut state = ListState::default();
    if self.items.is_empty() {
      state.select(None);
    } else {
      state.select(Some(self.selected.saturating_sub(offset)));
    }

    StatefulWidget::render(list, area, buf, &mut state);
  }
}
