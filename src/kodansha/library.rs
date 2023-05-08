use serde::Deserialize;

use crate::Volume;

#[derive(Deserialize, Default, Clone)]
pub struct Library {
    pub volumes: Vec<Volume>,
}
