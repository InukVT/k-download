use serde::Deserialize;

use crate::Volume;

#[derive(Deserialize)]
pub struct Library {
    pub volumes: Vec<Volume>,
}
