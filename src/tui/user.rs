use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

pub struct User {
    selected: Vec<usize>,
    list_state: ListState,
    user: crate::User,
}

impl User {
    pub fn render<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(
                [
                    Constraint::Percentage(40),
                    Constraint::Percentage(50),
                    Constraint::Percentage(10),
                ]
                .as_ref(),
            )
            .split(frame.size());

        let styled = Style::default();
        let highlight_style = Style::default().add_modifier(Modifier::BOLD);
        let list_items: Vec<ListItem> = self
            .user
            .library()
            .map(|library| {
                library
                    .volumes
                    .iter()
                    .enumerate()
                    .map(|(index, volume)| {
                        let span = Span::styled(
                            format!(
                                "{mark} {title}",
                                mark = match self.selected.contains(&index) {
                                    true => "[x]",
                                    false => "[ ]",
                                },
                                title = volume.volume_name
                            ),
                            styled.clone(),
                        );

                        ListItem::new(span)
                    })
                    .collect()
            })
            .unwrap_or(Vec::new());

        let block = Block::default().title("Library (L)").borders(Borders::ALL);
        let list = List::new(list_items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, chunks[0], &mut self.list_state);
    }
}

impl From<crate::User> for User {
    fn from(user: crate::User) -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        User {
            selected: Vec::new(),
            list_state,
            user,
        }
    }
}
