use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use anyhow::Result;
use epub_builder::EpubBuilder;
use epub_builder::EpubVersion;
use epub_builder::ZipLibrary;
use futures::future::join_all;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

use crate::kodansha::Page;

use super::page::RemotePage;

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

    pub async fn write_epub_to<W>(
        &self,
        token: &String,
        writer: &mut W,
        progress: Sender<(u16, u8)>,
    ) -> anyhow::Result<()>
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

        let page_requests = self.page_links(token).await?;
        let page_count = self.page_count as usize;

        for chunks in page_requests.chunks(10) {
            let chunks = chunks.iter().map(|(page_number, page)| async {
                page.write_to_epub(page_number, Arc::clone(&builder), token)
                    .await
            });

            let pages = join_all(chunks).await;
            sleep(Duration::from_millis(10)).await;

            for fun in pages.into_iter().map(|fun| fun.unwrap()) {
                let page = fun() + 1;
                let decimal: f32 = page as f32 / page_count as f32;
                let percent = decimal * 100.0f32;

                progress.send((self.id, percent as u8)).await?;
            }
        }

        let mut builder = builder.lock().unwrap();
        let _ = builder.generate(writer);
        Ok(())
    }

    pub async fn page_links(&self, token: &String) -> anyhow::Result<Vec<(usize, Page)>> {
        let volume_route = format!("https://api.kodansha.us/comic/{}/pages", self.id);
        let bearer = format!("Bearer {}", &token);

        let pages = reqwest::Client::new()
            .get(volume_route)
            .header("authorization", bearer.clone())
            .send()
            .await?
            .json::<Vec<RemotePage>>()
            .await?;

        let pages = join_all(pages.into_iter().map(|page| {
            let bearer = bearer.clone();
            async move {
                let page_number = page.page_number - 1;
                let url = format!(
                    "https://api.kodansha.us/comic/{volume}/pages/{page}",
                    volume = page.comic_id,
                    page = page_number
                );
                let page = reqwest::Client::new()
                    .get(url)
                    .header("authorization", bearer)
                    .send()
                    .await
                    .unwrap()
                    .json::<Page>()
                    .await
                    .unwrap();

                (page_number, page)
            }
        }))
        .await;

        Ok(pages)
    }
}
