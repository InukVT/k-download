use std::{
    path::PathBuf,
    rc::Rc,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

use crate::kodansha::Library;

pub struct Download {
    mode: Mode,
    url: Option<PathBuf>,
    library: Arc<Mutex<Option<Library>>>,
    selected: Rc<Mutex<Vec<usize>>>,
}

#[derive(Default)]
enum Mode {
    #[default]
    Normale,
    Download,
    DestinationSelection,
}

impl Download {
    pub fn new(library: Arc<Mutex<Option<Library>>>, selected: Rc<Mutex<Vec<usize>>>) -> Self {
        Download {
            mode: Mode::default(),
            url: None,
            library,
            selected,
        }
    }
    pub async fn prerender(&mut self) -> anyhow::Result<()> {
        match &self.mode {
            Mode::Download => {
                if None == self.url {
                    let dest = download_dir().await;

                    match dest {
                        Some(dest) => self.url = Some(dest),
                        None => self.mode = Mode::DestinationSelection,
                    }
                }

                Ok(())
            }
            Mode::DestinationSelection => Err(anyhow!("Not implemented")),
            _ => Ok(()),
        }
    }

    pub fn render<B>(&mut self, frame: &mut Frame<B>, rect: Rect)
    where
        B: Backend,
    {
        let library = self.library.lock().unwrap();
        let selected = self.selected.lock().unwrap();
        let styled = Style::default();
        let selected_items: Vec<ListItem> = library
            .clone()
            .unwrap_or_default()
            .volumes
            .iter()
            .enumerate()
            .filter_map(|(index, volume)| {
                if selected.contains(&index) {
                    Some(volume.clone())
                } else {
                    None
                }
            })
            .map(|volume| {
                let span = Span::styled(volume.volume_name, styled);

                ListItem::new(span)
            })
            .collect();

        let block = Block::default()
            .title("To Download (D)")
            .borders(Borders::ALL);

        let highlight_style = Style::default().add_modifier(Modifier::BOLD);
        let selection = List::new(selected_items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol("> ");

        frame.render_widget(selection, rect);
    }

    pub fn new_event(&mut self, _normal_mode: &mut bool, event: KeyEvent) -> bool {
        match (&mut self.mode, event.code) {
            (Mode::Normale, KeyCode::Char('d')) => {
                self.mode = Mode::Download;

                true
            }
            _ => false,
        }
    }

    pub fn get_selections(&self) -> Rc<Mutex<Vec<usize>>> {
        self.selected.clone()
    }
}

async fn download_dir() -> Option<PathBuf> {
    None
}

async fn set_download_dir() -> anyhow::Result<()> {
    Ok(())
}
