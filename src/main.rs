use std::io;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use k_download::tui::App;
use tui::backend::Backend;
use tui::{backend::CrosstermBackend, Terminal};

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

    ret?;

    Ok(())
}

async fn run_app<B>(terminal: &mut Terminal<B>, tick_rate: Duration) -> Result<()>
where
    B: Backend,
{
    let mut last_tick = Instant::now();

    let mut normal_mode = true;

    let mut app = App::default();

    loop {
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        app.prerender().await?;
        terminal.draw(|frame| app.render(frame))?;

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match (normal_mode, key.code) {
                    (true, KeyCode::Char('q')) => return Ok(()),
                    _ => app.new_event(&mut normal_mode, key),
                };
            }

            if last_tick.elapsed() >= tick_rate {
                //app.on_tick();
                last_tick = Instant::now();
            }
        };
    }
}
