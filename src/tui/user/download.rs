use std::{
    collections::HashMap,
    env::current_dir,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crossterm::event::{KeyCode, KeyEvent};
use futures_util::future::join_all;
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};
use tokio::{
    fs::{try_exists, File},
    io::AsyncWriteExt,
    sync::mpsc::{channel, Receiver, Sender},
    time::{sleep, Duration},
};

use crate::{
    kodansha::{
        user::{download_dir, set_download_dir},
        Library,
    },
    tui::tree::Tree,
    User, Volume,
};

pub struct Download {
    mode: Mode,
    destination: DownloadDestination,
    library: Arc<Mutex<Option<Library>>>,
    selected: Arc<Mutex<Vec<usize>>>,
    percents: HashMap<u16, u8>,
    tx: Sender<(u16, u8)>,
    rx: Receiver<(u16, u8)>,
}

#[derive(Debug)]
enum DownloadDestination {
    New(PathBuf),
    Current(PathBuf),
    None,
    Selecting,
}

#[derive(Default, Debug)]
enum Mode {
    #[default]
    Normal,
    Download,
    DestinationSelection((Tree, ListState)),
}

impl Download {
    pub fn new(library: Arc<Mutex<Option<Library>>>, selected: Arc<Mutex<Vec<usize>>>) -> Self {
        let (tx, rx) = channel(100);
        Download {
            mode: Mode::default(),
            destination: DownloadDestination::None,
            library,
            selected,
            tx,
            rx,
            percents: HashMap::default(),
        }
    }

    pub async fn prerender(&mut self, user: &mut User) -> anyhow::Result<()> {
        while let Ok((id, percent)) = self.rx.try_recv() {
            self.percents.insert(id, percent);
        }

        match &mut self.destination {
            DownloadDestination::New(new_url) => {
                let new_url = new_url.to_owned();
                set_download_dir(new_url.as_ref()).await?;

                self.destination = DownloadDestination::Current(new_url);
            }

            DownloadDestination::None => {
                let dir = download_dir().await?;
                if let Some(dir) = dir {
                    self.destination = DownloadDestination::Current(dir);
                }
            }

            _ => (),
        }

        match (&self.mode, &self.destination) {
            (Mode::Download, DownloadDestination::Selecting) => {
                let path = current_dir()?;
                self.destination = DownloadDestination::Current(path.clone());

                set_download_dir(&path).await?;

                let tree: Option<Tree> = path.ancestors().fold(None, |child, path| {
                    let ancestor = path.to_owned();
                    let mut contents: Vec<_> = fs::read_dir(path)
                        .ok()?
                        .filter_map(Result::ok)
                        .filter_map(|entry| {
                            let path = entry.path();

                            match &child {
                                None => Some(path),
                                Some(child) => {
                                    if *child.path() != path {
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

            (Mode::Download, DownloadDestination::Current(download_path)) => {
                let selected_items: Vec<Volume> = {
                    let library = self.library.lock().unwrap();
                    let selected = self.selected.lock().unwrap();

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
                        .collect()
                };

                let selectected_arc = Arc::new(Mutex::new(selected_items.clone()));
                let selected = self.selected.clone();
                let token = user.token().await?;
                let tx = self.tx.clone();

                let download_path = download_path.clone();
                tokio::spawn(async move {
                    for futs in selected_items.chunks(3) {
                        let futs = futs.iter().enumerate().map(|(count, volume)| {
                            let mut download_path = download_path.clone();
                            download_path.push(volume.volume_name.clone());
                            download_path.set_extension("epub");

                            let volume = volume.clone();
                            let token = token.clone();
                            let selected_items = selectected_arc.clone();

                            let tx = tx.clone();
                            let selected = selected.clone();

                            async move {
                                let _ = tx.send((volume.id, 0)).await;

                                let file = if try_exists(&download_path).await.unwrap_or(false) {
                                    File::open(download_path).await
                                } else {
                                    File::create(download_path).await
                                };

                                if let Ok(mut file) = file {
                                    let mut buffer: Vec<u8> = vec![];
                                    sleep(Duration::from_millis(10 * count as u64)).await;
                                    if volume.write_epub_to(&token, &mut buffer, tx).await.is_ok() {
                                        let _ = file.write_all(&buffer).await.is_ok();
                                    }

                                    if let Ok(mut selected_items) = selected_items.lock() {
                                        let index = selected_items
                                            .iter()
                                            .enumerate()
                                            .find_map(|(index, vol)| {
                                                if vol.id == volume.id {
                                                    Some(index)
                                                } else {
                                                    None
                                                }
                                            })
                                            .unwrap();

                                        selected_items.remove(index);
                                        selected.lock().unwrap().remove(index);
                                    }
                                }
                            }
                        });

                        join_all(futs).await;
                    }
                });

                self.mode = Mode::Normal;
            }
            _ => {}
        };

        Ok(())
    }

    pub fn render<B>(&mut self, frame: &mut Frame<B>, rect: Rect)
    where
        B: Backend,
    {
        match &mut self.mode {
            Mode::DestinationSelection((tree, state)) => {
                if let Some(items) = tree.list_items() {
                    if let (true, DownloadDestination::Current(url)) =
                        (state.selected().is_none(), &self.destination)
                    {
                        state.select(
                            items
                                .clone()
                                .into_iter()
                                .map(|(_, path)| path)
                                .enumerate()
                                .find_map(
                                    |(index, path)| if path == *url { Some(index) } else { None },
                                ),
                        )
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
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Max(3)])
                    .split(rect);

                let selected_items: Vec<ListItem> = {
                    let library = self.library.lock().unwrap();
                    let selected = self.selected.lock().unwrap();
                    let styled = Style::default();
                    let percent_style = Style::default().fg(Color::Green);

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
                            let percent = self
                                .percents
                                .get(&volume.id)
                                .map(|percent| {
                                    Span::styled(format!("[{}%] ", percent), percent_style)
                                })
                                .unwrap_or(Span::raw(""));

                            let span = Span::styled(volume.volume_name, styled);

                            let book = Spans::from(vec![percent, span]);

                            ListItem::new(book)
                        })
                        .collect()
                };

                let download_title = format!(
                    "Queue{}",
                    if selected_items.len() > 0 { " (D)" } else { "" }
                );
                let block = Block::default().title(download_title).borders(Borders::ALL);

                let highlight_style = Style::default().add_modifier(Modifier::BOLD);
                let selection = List::new(selected_items)
                    .block(block)
                    .highlight_style(highlight_style)
                    .highlight_symbol("> ");

                frame.render_widget(selection, chunks[0]);

                let block = Block::default()
                    .title("Destination (F)")
                    .borders(Borders::ALL);

                let text = Span::raw(match &self.destination {
                    DownloadDestination::Current(path) => path.to_str().unwrap_or("").to_string(),
                    _ => "Press F to select destination".to_string(),
                });
                let text = vec![(ListItem::new(text))];

                let text = List::new(text).block(block);

                frame.render_widget(text, chunks[1]);
            }
        }
    }

    pub fn new_event(&mut self, normal_mode: &mut bool, event: KeyEvent) -> bool {
        match (&mut self.mode, event.code) {
            (Mode::Normal, KeyCode::Char('f')) => {
                self.mode = Mode::Download;
                self.destination = DownloadDestination::Selecting;

                *normal_mode = false;

                true
            }

            (Mode::Normal, KeyCode::Char('d')) => {
                let selected = { self.selected.lock().unwrap().len() > 0 };
                if let (DownloadDestination::Current(_), true) = (&mut self.destination, selected) {
                    self.mode = Mode::Download;

                    true
                } else {
                    false
                }
            }

            (Mode::DestinationSelection((tree, state)), KeyCode::Enter) => {
                if let Some(new_url) = state
                    .selected()
                    .and_then(|index| {
                        let mut index = index;
                        tree.get(&mut index)
                    })
                    .map(|node| node.path())
                    .map(|path| path.to_owned())
                    .map(DownloadDestination::New)
                {
                    self.destination = new_url;
                }

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

            (
                Mode::DestinationSelection((tree, state)),
                KeyCode::Char('o') | KeyCode::Char(' '),
            ) => {
                if let Some(node) = state.selected().and_then(|index| {
                    let mut index = index;
                    tree.get(&mut index)
                }) {
                    node.toggl();

                    let path = node.path();
                    let contents: Option<Vec<_>> = match fs::read_dir(path) {
                        Ok(dir) => Some(
                            dir.filter_map(Result::ok)
                                .map(|entry| entry.path())
                                .map(Tree::new)
                                .collect(),
                        ),
                        Err(_) => None,
                    };

                    node.set_children_optional(contents);
                }

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

    pub fn get_selections(&self) -> Arc<Mutex<Vec<usize>>> {
        self.selected.clone()
    }
}
