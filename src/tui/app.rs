use anyhow::Ok;
use crossterm::event::KeyEvent;
use tui::{backend::Backend, Frame};

use crate::Credentials;

use super::login::LoginScreen;

pub struct App {
    state: State,
}

enum State {
    User(super::user::User),
    NoUser(LoginScreen),
}

impl Default for App {
    fn default() -> App {
        App {
            state: State::NoUser(LoginScreen::default()),
        }
    }
}

impl App {
    pub async fn prerender(&mut self) -> anyhow::Result<()> {
        if let State::NoUser(login_screen) = &mut self.state {
            match Credentials::from_config().await {
                Result::Ok(credentials) => {
                    let mut user = credentials.login().await?;
                    user.load_library().await?;
                    self.state = State::User(user.into());
                }
                _ => {
                    if let Some(credentials) = login_screen.get_credentials() {
                        self.state = State::User(credentials.login().await?.into())
                    };
                }
            }
        }

        Ok(())
    }

    pub fn render<B>(&mut self, frame: &mut Frame<B>)
    where
        B: Backend,
    {
        match &mut self.state {
            State::NoUser(login_screen) => login_screen.render(frame),
            State::User(user) => user.render(frame),
        }
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        match &mut self.state {
            State::NoUser(login_screen) => login_screen.new_event(normal_mode, event),
            State::User(user) => user.new_event(normal_mode, event),
        }
    }
}
