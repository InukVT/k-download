use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Layout},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::Credentials;

#[derive(Default)]
pub struct LoginScreen {
    username: String,
    password: String,
    submit: bool,
    state: State,
}

#[derive(Clone, Default)]
enum State {
    #[default]
    Normal,
    Username,
    Password,
}

impl LoginScreen {
    pub fn render<B>(&mut self, frame: &mut Frame<B>)
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

    pub fn get_credentials(&self) -> Option<Credentials> {
        if !self.submit {
            return None;
        }
        let username = self.username.clone();
        let password = self.password.clone();

        Some(Credentials::new(username, password))
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
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

        true
    }
}
