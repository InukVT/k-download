use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::kodansha::library::LibraryItem;

pub struct User {
    selected: Vec<usize>,
    list_state: ListState,
    user: crate::User,
    highlight_mode: bool,
}

impl User {
    pub fn render<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(frame.size());

        let styled = Style::default();
        let highlight_style = Style::default().add_modifier(Modifier::BOLD);

        let library = self.user.library().unwrap_or_default();

        let list_items: Vec<ListItem> = library
            .clone()
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
                    styled,
                );

                ListItem::new(span)
            })
            .collect();

        let selected_items: Vec<ListItem> = library
            .volumes
            .iter()
            .enumerate()
            .filter(|(index, _volume)| self.selected.contains(index))
            .map(|(_index, volume)| {
                let span = Span::styled(volume.volume_name.clone(), styled);

                ListItem::new(span)
            })
            .collect();

        let block = Block::default().title("Library (L)").borders(Borders::ALL);
        let list = List::new(list_items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol("> ");

        let block = Block::default()
            .title("To Download (D)")
            .borders(Borders::ALL);
        let selection = List::new(selected_items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol("> ");

        let book_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(panels[0]);

        frame.render_stateful_widget(list, book_chunks[0], &mut self.list_state);
        frame.render_widget(selection, book_chunks[1]);

        let block = Block::default().title("Book Info").borders(Borders::ALL);

        let text: Vec<Spans> = if let Some(volume) = self.user.library().and_then(|library| {
            self.list_state
                .selected()
                .and_then(|index| Some(library.volumes.get(index)?.clone()))
        }) {
            vec![
                Spans::from(vec![Span::raw("Series:")]),
                Spans::from(vec![Span::raw(volume.series_name)]),
                Spans::from(vec![Span::raw("")]),
                Spans::from(vec![Span::raw("Volume:")]),
                Spans::from(vec![Span::raw(volume.volume_name)]),
                Spans::from(vec![Span::raw("")]),
                Spans::from(vec![Span::raw("Description:")]),
                Spans::from(vec![Span::raw(volume.description)]),
            ]
        } else {
            Vec::default()
        };

        let list = Paragraph::new(text).block(block);

        frame.render_widget(list, panels[1]);
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        match (&mut self.highlight_mode, event.code) {
            (false, KeyCode::Char('l')) => {
                self.list_state.select(Some(0));
                *normal_mode = false;
                self.highlight_mode = true;
                true
            }
            (true, KeyCode::Char('j') | KeyCode::Down) => {
                if let (Some(library), Some(selected)) =
                    (self.user.library(), self.list_state.selected())
                {
                    let volumes = library.volumes;
                    let count = volumes.len();
                    let new_selection = selected + 1;
                    self.list_state.select(Some(new_selection % count));
                } else if let (Some(_), None) = (self.user.library(), self.list_state.selected()) {
                    self.list_state.select(Some(0));
                };

                true
            }
            (true, KeyCode::Char('k') | KeyCode::Up) => {
                if let (Some(library), Some(selected)) =
                    (self.user.library(), self.list_state.selected())
                {
                    let volumes = library.volumes;
                    let count = volumes.len();
                    let new_selection = if selected >= 1 {
                        selected - 1
                    } else {
                        count - 1
                    };
                    self.list_state.select(Some(new_selection));
                } else if let (Some(_), None) = (self.user.library(), self.list_state.selected()) {
                    self.list_state.select(Some(0));
                };

                true
            }
            (true, KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l')) => {
                self.list_state.select(None);
                self.highlight_mode = false;
                *normal_mode = true;

                true
            }
            (true, KeyCode::Char(' ') | KeyCode::Char('a')) => {
                if let Some(selected) = self.list_state.selected() {
                    if self.selected.contains(&selected) {
                        let index = self.selected.iter().position(|x| *x == selected).unwrap();
                        self.selected.remove(index);
                    } else {
                        self.selected.push(selected);
                    }
                }

                true
            }

            _ => false,
        }
    }
}

impl From<crate::User> for User {
    fn from(user: crate::User) -> Self {
        let list_state = ListState::default();
        User {
            selected: Vec::new(),
            list_state,
            user,
            highlight_mode: false,
        }
    }
}
