use std::collections::HashMap;

use futures_util::future::join_all;
use serde::Deserialize;

use crate::{User, Volume};

use super::Library;

#[derive(Deserialize, Debug, Clone)]
pub struct Series {
    pub id: u16,
    pub title: String,
    pub genres: Vec<String>,
    pub volumes: Option<Vec<Volume>>,
}

impl Series {
    pub async fn from_library(library: &Library, user: &mut User) -> Vec<Series> {
        let mut series: HashMap<u16, Vec<Volume>> = HashMap::new();
        for volume in &library.volumes {
            let mut current_series = match series.get(&volume.series_id) {
                Some(series) => series.to_owned(),
                None => Vec::default(),
            };

            current_series.push(volume.to_owned());

            series.insert(volume.series_id, current_series);
        }

        let token = user.token().await.unwrap();
        join_all(series.iter().map(move |(series_id, volumes)| {
            let series_route = format!("https://api.kodansha.us/series/{}/", series_id);
            let volumes = volumes.to_owned();
            let bearer = format!("Bearer {}", token.clone());

            tokio::spawn(async {
                let mut series = reqwest::Client::new()
                    .get(series_route)
                    .header("authorization", bearer)
                    .send()
                    .await
                    .unwrap()
                    .json::<Series>()
                    .await
                    .unwrap();

                series.volumes = Some(volumes);

                series
            })
        }))
        .await
        .into_iter()
        .map(|series| series.unwrap())
        .collect()
    }
}
