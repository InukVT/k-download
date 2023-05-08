use crossterm::event::KeyEvent;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

use crate::{Credentials, User};

use super::login::LoginScreen;

pub struct App {
    state: State,
    selected: Vec<usize>,
    list_state: ListState,
}

enum State {
    User(User),
    NoUser(LoginScreen),
}

impl App {
    pub fn new() -> App {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        App {
            state: State::NoUser(LoginScreen::default()),
            selected: Vec::new(),
            list_state,
        }
    }

    pub async fn render<B>(&mut self, terminal: &mut Terminal<B>) -> anyhow::Result<()>
    where
        B: Backend,
    {
        match &mut self.state {
            State::NoUser(login_screen) => {
                let credentials = Credentials::from_config().await;
                if let Ok(credentials) = credentials {
                    let mut user = credentials.login().await?;
                    user.load_library().await?;
                    self.state = State::User(user);
                    terminal.clear()?;
                } else {
                    terminal.draw(|frame| login_screen.render(frame))?;

                    if let Some(credentials) = login_screen.get_credentials() {
                        self.state = State::User(credentials.login().await?)
                    };
                }
            }
            State::User(user) => {
                terminal.draw(|frame| {
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
                    let list_items: Vec<ListItem> = user
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
                })?;
            }
        };
        Ok(())
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        match &mut self.state {
            State::NoUser(login_screen) => login_screen.new_event(normal_mode, event),
            _ => false,
        }
    }
}
