use anyhow::Result;
use futures::future::join_all;
use serde::Deserialize;

use crate::kodansha::Page;
use crate::kodansha::User;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volume {
    pub series_name: String,
    pub volume_name: String,
    pub volume_number: u8,
    pub page_count: u16,
    pub description: String,
    pub id: u16,
}

impl Volume {
    pub async fn get(url_id: u16) -> Result<Volume> {
        let volume_route = format!("https://api.kodansha.us/comic/{}/", url_id);
        let volume = reqwest::get(volume_route)
            .await
            .unwrap()
            .json::<Volume>()
            .await
            .unwrap();

        Ok(volume)
    }

    pub async fn page_links(self, user: &User) -> Box<dyn Iterator<Item = (usize, Page)>> {
        let volume_route = format!("https://api.kodansha.us/comic/{}", self.id);

        Box::new(
            join_all((0..self.page_count + 1).map(|index| {
                let volume_route = volume_route.clone();
                let token = user.token.clone();

                tokio::spawn(async move {
                    let page_route = format!("{}/pages/{page}", volume_route, page = index);

                    reqwest::Client::new()
                        .get(page_route)
                        .header("authorization", format!("Bearer {}", token))
                        .send()
                        .await
                        .unwrap()
                        .json::<Page>()
                        .await
                        .unwrap()
                })
            }))
            .await
            .into_iter()
            .map(|val| val.unwrap())
            .enumerate(),
        )
    }
}
