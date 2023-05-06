use std::io;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use futures_util::FutureExt;
use kodansha_downloader::Credentials;
use kodansha_downloader::Volume;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::backend::Backend;
use tui::style::Color;
use tui::style::Modifier;
use tui::style::Style;
use tui::text::Span;
use tui::widgets::List;
use tui::widgets::ListItem;
use tui::widgets::ListState;
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders, Widget},
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
    run_app(&mut terminal, tick_rate).await?;

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

    Ok(())
}

async fn run_app<B>(terminal: &mut Terminal<B>, tick_rate: Duration) -> Result<()>
where
    B: Backend,
{
    let user = Credentials::from_config(
        async {
            let mut username: Option<String> = None;
            let mut password: Option<String> = None;
            loop {
                if let (Some(username), Some(password)) = (username.clone(), password.clone()) {
                    return Ok(Credentials::new(username, password));
                }
            }
        }
        .boxed(),
    )
    .await?;
    let mut last_tick = Instant::now();
    let mut state = ListState::default();

    let values = vec!["Line 1", "Line 2", "Line 3"];
    let mut selected: Vec<usize> = vec![];
    state.select(Some(0));
    loop {
        terminal.draw(|f| {
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
                .split(f.size());

            let styled = Style::default();
            let highlight_style = Style::default().add_modifier(Modifier::BOLD);
            let list_items: Vec<ListItem> = values
                .iter()
                .enumerate()
                .map(|(index, txt)| {
                    let span = Span::styled(
                        format!(
                            "{mark} {title}",
                            mark = match selected.contains(&index) {
                                true => "[x]",
                                false => "[ ]",
                            },
                            title = txt
                        ),
                        styled.clone(),
                    );

                    ListItem::new(span)
                })
                .collect();

            let block = Block::default().title("Library (L)").borders(Borders::ALL);
            let list = List::new(list_items)
                .block(block)
                .highlight_style(highlight_style)
                .highlight_symbol("> ");

            f.render_stateful_widget(list, chunks[0], &mut state);
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => state.select(
                        state
                            .selected()
                            .map(|num| (num + 1) % values.iter().count()),
                    ),
                    KeyCode::Char('k') | KeyCode::Up => {
                        state.select(state.selected().map(|num| match num <= 0 {
                            true => values.iter().count() - 1,
                            false => num - 1,
                        }))
                    }
                    KeyCode::Char(' ') => {
                        state.selected().map(|current| {
                            match selected.iter().position(|&test| test == current) {
                                Some(index) => {
                                    selected.remove(index);
                                }
                                None => {
                                    selected.push(current);
                                }
                            }
                        });
                    }

                    _ => {}
                }
            }

            if last_tick.elapsed() >= tick_rate {
                //app.on_tick();
                last_tick = Instant::now();
            }
        }
    }
}
