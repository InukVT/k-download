use std::sync::{Arc, Mutex};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::utils::ToDedup;

use super::Download;

pub struct User {
    selected: Arc<Mutex<Vec<usize>>>,

    list_state: ListState,
    user: crate::User,
    download_tab: Download,
    mode: Mode,
}

#[derive(Default)]
enum Mode {
    #[default]
    Normal,
    Highlight,
    Download,
}

impl User {
    pub async fn prerender(&mut self) -> anyhow::Result<()> {
        self.download_tab.prerender(&self.user).await
    }

    pub fn render<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(frame.size());

        let styled = Style::default();

        let list_items: Vec<ListItem> = {
            let selected = self.selected.lock().unwrap();

            let library = self.user.library();
            let library = library.lock().unwrap();
            let library = library.clone().unwrap_or_default();

            library
                .volumes
                .iter()
                .enumerate()
                .map(|(index, volume)| {
                    let span = Span::styled(
                        format!(
                            "{mark} {title}",
                            mark = match selected.contains(&index) {
                                true => "[x]",
                                false => "[ ]",
                            },
                            title = volume.volume_name
                        ),
                        styled,
                    );

                    ListItem::new(span)
                })
                .collect()
        };

        let highlight_style = Style::default().add_modifier(Modifier::BOLD);

        let block = Block::default().title("Library (L)").borders(Borders::ALL);
        let list = List::new(list_items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol(">");

        let book_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(panels[0]);

        frame.render_stateful_widget(list, book_chunks[0], &mut self.list_state);
        self.download_tab.render(frame, book_chunks[1]);

        let block = Block::default().title("Book Info").borders(Borders::ALL);

        let library = self.user.library();
        let library = library.lock();
        let text: Vec<Spans> = match library.ok().and_then(|library| {
            library.clone().and_then(|library| {
                self.list_state
                    .selected()
                    .and_then(|index| Some(library.volumes.get(index)?.clone()))
            })
        }) {
            Some(volume) => {
                let mut description = {
                    let escaped = html_escape::decode_html_entities(volume.description.as_str());

                    let description: Vec<Spans> = escaped
                        .replace('\r', "")
                        .split('\n')
                        .map(|line| if line == " " { "" } else { line })
                        .map(|line| line.to_owned())
                        .dedup()
                        .map(|line| Spans::from(vec![Span::raw(line)]))
                        .collect();

                    description
                };

                let mut ret = vec![
                    Spans::from(vec![Span::raw("Series:")]),
                    Spans::from(vec![Span::raw(volume.series_name)]),
                    Spans::from(vec![Span::raw("")]),
                    Spans::from(vec![Span::raw("Volume:")]),
                    Spans::from(vec![Span::raw(volume.volume_name)]),
                    Spans::from(vec![Span::raw("")]),
                    Spans::from(vec![Span::raw("Description:")]),
                ];

                ret.append(&mut description);

                ret
            }
            None => Vec::default(),
        };

        let list = Paragraph::new(text).block(block).wrap(Wrap { trim: true });

        frame.render_widget(list, panels[1]);
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        let library = self.user.library();
        let library = library.lock().unwrap();
        match (&mut self.mode, event.code) {
            (Mode::Normal, KeyCode::Char('l')) => {
                self.list_state.select(Some(0));
                *normal_mode = false;
                self.mode = Mode::Highlight;
                true
            }

            (Mode::Normal, KeyCode::Char('d') | KeyCode::Char('f')) => {
                self.mode = Mode::Download;
                self.download_tab.new_event(normal_mode, event);

                true
            }

            (Mode::Download, KeyCode::Enter) => {
                self.mode = Mode::Normal;
                self.download_tab.new_event(normal_mode, event);

                true
            }

            (Mode::Highlight, KeyCode::Char('j') | KeyCode::Down) => {
                match (Option::as_ref(&library), self.list_state.selected()) {
                    (Some(library), Some(selected)) => {
                        let volumes = library.volumes.clone();
                        let count = volumes.len();
                        let new_selection = selected + 1;
                        self.list_state.select(Some(new_selection % count));
                    }
                    (Some(_), None) => {
                        self.list_state.select(Some(0));
                    }
                    _ => (),
                };

                true
            }

            (Mode::Highlight, KeyCode::Char('k') | KeyCode::Up) => {
                match (Option::as_ref(&library), self.list_state.selected()) {
                    (Some(library), Some(selected)) => {
                        let volumes = library.volumes.clone();
                        let count = volumes.len();
                        let new_selection = if selected >= 1 {
                            selected - 1
                        } else {
                            count - 1
                        };
                        self.list_state.select(Some(new_selection));
                    }
                    (Some(_), None) => {
                        self.list_state.select(Some(0));
                    }
                    _ => (),
                };
                true
            }

            (
                Mode::Highlight | Mode::Download,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('l'),
            ) => {
                self.list_state.select(None);
                self.mode = Mode::Normal;
                *normal_mode = true;

                true
            }

            (Mode::Highlight, KeyCode::Char(' ') | KeyCode::Char('a')) => {
                let mut selected_item = self.selected.lock().unwrap();
                if let Some(selected) = self.list_state.selected() {
                    if selected_item.contains(&selected) {
                        let index = selected_item.iter().position(|x| *x == selected).unwrap();
                        selected_item.remove(index);
                    } else {
                        selected_item.push(selected);
                    }
                }

                true
            }

            _ => self.download_tab.new_event(normal_mode, event),
        }
    }
}

impl From<crate::User> for User {
    fn from(user: crate::User) -> Self {
        let list_state = ListState::default();

        let library = user.library();
        let download_tab = Download::new(library, Arc::default());
        let selected = download_tab.get_selections();

        User {
            selected,
            list_state,
            user,
            mode: Mode::default(),
            download_tab,
        }
    }
}
