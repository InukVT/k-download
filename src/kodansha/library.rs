use serde::Deserialize;

use crate::Volume;

#[derive(Deserialize, Default, Clone)]
pub struct Chapter;

#[derive(Deserialize, Clone)]
pub enum LibraryItem {
    Chapter(Chapter),
    Volume(Volume),
}

#[derive(Deserialize, Default, Clone)]
pub struct Library {
    pub volumes: Vec<Volume>,
}
