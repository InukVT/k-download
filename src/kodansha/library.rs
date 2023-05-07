use serde::Deserialize;

use crate::Volume;

#[derive(Deserialize, Clone)]
pub struct Library {
    pub volumes: Vec<Volume>,
}
