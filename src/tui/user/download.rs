use std::{
    env::current_dir,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    backend::Backend,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::{kodansha::Library, tui::tree::Tree};

pub struct Download {
    mode: Mode,
    url: Option<PathBuf>,
    library: Arc<Mutex<Option<Library>>>,
    selected: Rc<Mutex<Vec<usize>>>,
}

#[derive(Default)]
enum Mode {
    #[default]
    Normal,
    Download,
    DestinationSelection((Tree, ListState)),
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
                if self.url.is_none() {
                    let dest = download_dir().await;

                    match dest {
                        Some(dest) => self.url = Some(dest),
                        None => {
                            let path = current_dir()?;
                            self.url = Some(path.clone());

                            set_download_dir(&path).await?;

                            self.url = Some(path.clone());

                            let tree: Option<Tree> = path.ancestors().fold(None, |child, path| {
                                let ancestor = path.to_owned();
                                let mut contents: Vec<_> = fs::read_dir(&path)
                                    .ok()?
                                    .filter_map(Result::ok)
                                    .filter_map(|entry| {
                                        let path = entry.path();

                                        match &child {
                                            None => Some(path),
                                            Some(child) => {
                                                if child.path().to_owned() != path {
                                                    Some(path)
                                                } else {
                                                    None
                                                }
                                            }
                                        }
                                    })
                                    .map(Tree::new)
                                    .collect();

                                if let Some(child) = child {
                                    contents.push(child);
                                }

                                let mut parent_tree = Tree::new(ancestor);

                                parent_tree.set_children(contents);
                                parent_tree.open();

                                Some(parent_tree)
                            });

                            let state = ListState::default();

                            self.mode = Mode::DestinationSelection((tree.unwrap(), state));
                        }
                    }
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn render<B>(&mut self, frame: &mut Frame<B>, rect: Rect)
    where
        B: Backend,
    {
        match &mut self.mode {
            Mode::DestinationSelection((tree, state)) => {
                if let Some(items) = tree.list_items() {
                    match (state.selected().is_none(), self.url.clone()) {
                        (true, Some(url)) => state.select(
                            items
                                .clone()
                                .into_iter()
                                .map(|(_, path)| path)
                                .enumerate()
                                .find_map(
                                    |(index, path)| if path == url { Some(index) } else { None },
                                ),
                        ),
                        _ => {}
                    }

                    let block = Block::default()
                        .title("Select Destination (Enter)")
                        .borders(Borders::ALL);

                    let items: Vec<_> = items.into_iter().map(|(tree, _)| tree).collect();
                    let highlight_style = Style::default().add_modifier(Modifier::BOLD);
                    let selection = List::new(items)
                        .block(block)
                        .highlight_style(highlight_style)
                        .highlight_symbol("> ");

                    frame.render_stateful_widget(selection, rect, state);
                }
            }
            Mode::Download | Mode::Normal => {
                let selected_items: Vec<ListItem> = {
                    let library = self.library.lock().unwrap();
                    let selected = self.selected.lock().unwrap();
                    let styled = Style::default();

                    library
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
                        .collect()
                };

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
        }
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        match (&mut self.mode, event.code) {
            (Mode::Normal, KeyCode::Char('d')) => {
                self.mode = Mode::Download;
                *normal_mode = false;

                true
            }
            (Mode::DestinationSelection(_), KeyCode::Enter) => {
                self.mode = Mode::Normal;
                *normal_mode = true;

                true
            }

            (Mode::DestinationSelection((tree, state)), KeyCode::Char('j') | KeyCode::Down) => {
                match (tree.list_items(), state.selected()) {
                    (Some(tree), Some(selected)) => {
                        let count = tree.len();
                        let new_selection = selected + 1;
                        state.select(Some(new_selection % count));
                    }
                    (Some(_), None) => {
                        state.select(Some(0));
                    }
                    _ => (),
                };

                true
            }
            (Mode::DestinationSelection((tree, state)), KeyCode::Char('k') | KeyCode::Up) => {
                match (tree.list_items(), state.selected()) {
                    (Some(tree), Some(selected)) => {
                        let count = tree.len();
                        let new_selection = if selected >= 1 {
                            selected - 1
                        } else {
                            count - 1
                        };
                        state.select(Some(new_selection));
                    }
                    (Some(_), None) => {
                        state.select(Some(0));
                    }
                    _ => (),
                };
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

async fn set_download_dir(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
