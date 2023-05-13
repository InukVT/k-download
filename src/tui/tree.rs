use std::path::{Path, PathBuf};

use ratatui::{
    text::{Span, Spans},
    widgets::ListItem,
};

#[derive(Clone)]
pub struct Tree {
    path: PathBuf,
    children: Option<Vec<Tree>>,
    open: bool,
}

impl Tree {
    pub fn new(title: PathBuf) -> Tree {
        Tree {
            path: title,
            children: None,
            open: false,
        }
    }

    pub fn set_children(&mut self, children: Vec<Tree>) {
        self.children = Some(children);
    }

    pub fn set_children_optional(&mut self, children: Option<Vec<Tree>>) {
        self.children = children;
    }

    pub fn list_items(&self) -> Option<Vec<(ListItem, PathBuf)>> {
        if !self.path.is_dir() {
            return None;
        }

        let title = if *Path::new("/") == self.path {
            "/"
        } else {
            self.path.file_name()?.to_str()?
        }
        .to_owned();

        let count = self.path.ancestors().count();

        let mut whitespace: Vec<_> = (1..count).map(|_indent| Span::raw("  ")).collect();
        let title = Span::raw(title);

        whitespace.push(title);

        let mut spans = vec![(ListItem::new(Spans::from(whitespace)), self.path.clone())];

        if let (Some(children), true) = (&self.children, self.open) {
            for child in children {
                let child_item = child.list_items();
                if let Some(mut child_item) = child_item {
                    spans.append(&mut child_item);
                }
            }
        }

        Some(spans)
    }

    pub fn get(&mut self, index: &mut usize) -> Option<&mut Self> {
        if !self.path.is_dir() {
            return None;
        }

        if *index == 0 {
            return Some(self);
        }

        *index -= 1;

        if let (Some(children), true) = (&mut self.children, self.open) {
            for child in children.iter_mut() {
                if let Some(node) = child.get(index) {
                    return Some(node);
                }
            }
        }

        None
    }

    pub fn toggl(&mut self) {
        self.open = !self.open;
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}
