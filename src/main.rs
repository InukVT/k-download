use std::io;
use std::time::Duration;
use std::time::Instant;

use anyhow::{anyhow, Result};
use crossterm::event::KeyEvent;
use kodansha_downloader::Credentials;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use kodansha_downloader::User;
use tui::backend::Backend;
use tui::style::Modifier;
use tui::style::Style;
use tui::text::Span;
use tui::widgets::List;
use tui::widgets::ListItem;
use tui::widgets::ListState;
use tui::widgets::Paragraph;
use tui::Frame;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
    Terminal,
};

#[tokio::main]
async fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let ret = run_app(&mut terminal, tick_rate).await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    //let user = Credentials::from_config()?.login().await?;

    //let volume = Volume::get(10).await?;

    //let mut stdout = io::stdout();
    //volume.write_epub_to(&user, &mut stdout).await?;

    if let Err(err) = ret {
        print!("Error: {}", err);
    }

    Ok(())
}

#[derive(Clone)]
enum State {
    Normal,
    Username,
    Password,
}

impl Default for State {
    fn default() -> Self {
        State::Normal
    }
}

#[derive(Default)]
struct LoginScreen {
    username: String,
    password: String,
    submit: bool,
    state: State,
}

impl LoginScreen {
    fn render<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        let chunks = Layout::default()
            .margin(1)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.size());

        let block = Block::default().title("Username (U)").borders(Borders::ALL);
        let text_field = Paragraph::new(self.username.clone()).block(block);
        frame.render_widget(text_field, chunks[0]);

        let block = Block::default().title("Password (P)").borders(Borders::ALL);
        let text_field = Paragraph::new(self.password.clone()).block(block);
        frame.render_widget(text_field, chunks[1]);
    }

    fn get_credentials(&self) -> Option<Credentials> {
        if !self.submit {
            return None;
        }
        let username = self.username.clone();
        let password = self.password.clone();

        return Some(Credentials::new(username, password));
    }

    fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        match (event.code, self.state.clone()) {
            (KeyCode::Char(char), State::Username) => self.username.push(char),
            (KeyCode::Char(char), State::Password) => self.password.push(char),
            (KeyCode::Char('u'), State::Normal) => {
                self.state = State::Username;
                *normal_mode = false;
            }
            (KeyCode::Char('p'), State::Normal) => {
                self.state = State::Password;
                *normal_mode = false;
            }
            (KeyCode::Backspace, State::Username) => {
                let _ = self.username.pop();
            }

            (KeyCode::Backspace, State::Password) => {
                let _ = self.password.pop();
            }
            (KeyCode::Esc, State::Password | State::Username) => {
                self.state = State::Normal;
                *normal_mode = true;
            }

            (KeyCode::Enter, State::Normal) => self.submit = true,
            _ => return false,
        };
        return true;
    }
}

enum RuntimeState {
    User(User),
    NoUser(LoginScreen),
}

async fn run_app<B>(terminal: &mut Terminal<B>, tick_rate: Duration) -> Result<()>
where
    B: Backend,
{
    let mut runtime_state = RuntimeState::NoUser(LoginScreen::default());

    let mut last_tick = Instant::now();
    let mut state = ListState::default();
    state.select(Some(0));

    let selected = vec![];

    let mut normal_mode = true;
    let mut key_event = None;

    loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        match &mut runtime_state {
            RuntimeState::NoUser(login_screen) => {
                let credentials = Credentials::from_config().await;
                if let Ok(credentials) = credentials {
                    let mut user = credentials.login().await?;
                    user.load_library().await?;
                    runtime_state = RuntimeState::User(user);
                    terminal.clear();
                } else {
                    terminal.draw(|frame| login_screen.render(frame))?;

                    if let Some(event) = key_event {
                        login_screen.new_event(&mut normal_mode, event);
                    }

                    if let Some(credentials) = login_screen.get_credentials() {
                        runtime_state = RuntimeState::User(credentials.login().await?)
                    };
                }
            }
            RuntimeState::User(user) => {
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
                                            mark = match selected.contains(&index) {
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

                    frame.render_stateful_widget(list, chunks[0], &mut state);
                })?;
            }
        };

        key_event = None;
        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match (normal_mode, key.code) {
                    (true, KeyCode::Char('q')) => return Ok(()),
                    _ => key_event = Some(key),
                }
            }

            if last_tick.elapsed() >= tick_rate {
                //app.on_tick();
                last_tick = Instant::now();
            }
        };
    }
}
