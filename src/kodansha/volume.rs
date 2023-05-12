use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use epub_builder::EpubBuilder;
use epub_builder::EpubVersion;
use epub_builder::ZipLibrary;
use futures::future::join_all;
use serde::Deserialize;

use crate::kodansha::Page;
use crate::kodansha::User;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Volume<VolumeName = String> {
    pub series_name: String,
    pub volume_name: VolumeName,
    pub volume_number: u8,
    pub page_count: u16,
    pub description: String,
    pub id: u16,
    pub series_id: u16,
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

    pub async fn write_epub_to<W>(&self, user: User, writer: &mut W) -> anyhow::Result<()>
    where
        W: std::io::Write,
    {
        let zip = ZipLibrary::new().unwrap();
        let mut builder = EpubBuilder::new(zip).unwrap();

        builder
            .metadata("title", self.volume_name.clone())
            .unwrap()
            .metadata(
                "description",
                self.description.clone().replace("rsquo", "apos"),
            )
            //.unwrap()
            //.metadata("series", volume.series_name.clone())
            .unwrap()
            .metadata("subject", "Manga")
            .unwrap()
            .epub_version(EpubVersion::V30);

        let builder = Arc::new(Mutex::new(builder));

        let page_requests = self.page_links(&user).await;

        for chunks in page_requests.chunks(10) {
            let chunks = chunks.into_iter().map(|(page_number, page)| {
                let user = user.clone();
                let builder = builder.clone();
                tokio::spawn(async move {
                    (
                        page_number,
                        page.write_to_epub(page_number, builder, &user)
                            .await
                            .unwrap(),
                    )
                })
            });

            let mut pages = join_all(chunks).await;

            pages.sort_by_key(|r| match r {
                Ok(res) => res.0,
                Err(_) => panic!(),
            });

            for (_, fun) in pages.into_iter().map(|r| r.unwrap()) {
                fun();
            }
        }
        let mut builder = builder.lock().unwrap();
        let _ = builder.generate(writer);
        Ok(())
    }

    pub async fn page_links(&self, user: &User) -> Vec<(usize, Page)> {
        let volume_route = format!("https://api.kodansha.us/comic/{}", self.id);

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
        .enumerate()
        .collect()
    }
}
