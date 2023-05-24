use std::sync::{Arc, Mutex};

use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};
use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct Page {
    pub url: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemotePage {
    pub page_number: usize,
    #[serde(alias = "comicID")]
    pub comic_id: usize,
}

impl Page {
    fn image_template(page: usize, path: String) -> String {
        format!(
            "\
<?xml version='1.0' encoding='utf-8'?>\n\
<html xmlns=\"http://www.w3.org/1999/xhtml\">\n\
  <head>\n\
    <title>Page #{page}</title>\n\
    <meta http-equiv=\"Content-Type\" content=\"text/html; charset=utf-8\"/>\n\
  </head>\n\
  <body>\n\
    <img src=\"{path}\" alt=\"comic page #{page}\" />\n\
  </body>\n\
</html>\
",
            page = page,
            path = path
        )
    }

    pub async fn stream(&self, token: &String) -> anyhow::Result<Box<[u8]>> {
        Ok(reqwest::Client::new()
            .get(self.url.clone())
            .header("authorization", format!("Bearer {}", token))
            .send()
            .await?
            .bytes()
            .await?
            .as_ref()
            .to_owned()
            .into_boxed_slice())
    }

    pub async fn write_to_epub(
        &self,
        page_number: &usize,
        builder: Arc<Mutex<EpubBuilder<ZipLibrary>>>,
        token: &String,
    ) -> anyhow::Result<Box<dyn FnOnce() -> usize + Send + 'static>> {
        let (file_name, title, reference_type) = match page_number {
            0 => (
                "cover.jpeg".to_string(),
                "Cover".to_string(),
                ReferenceType::Cover,
            ),
            _ => (
                format!("page-{}.jpeg", page_number),
                format!("Page {}", page_number),
                ReferenceType::Text,
            ),
        };

        let image_path = format!("images/{}", file_name);
        let page_number = *page_number;
        let page_path = format!("page-{}.xhtml", page_number);

        let stream = self.stream(token).await?;

        {
            let mut builder = builder.lock().unwrap();
            match reference_type {
                ReferenceType::Cover => {
                    (*builder).add_cover_image(image_path.clone(), stream.as_ref(), "image/jpeg")
                }

                _ => (*builder).add_resource(image_path.clone(), stream.as_ref(), "image/jpeg"),
            }
            .unwrap();
        }

        Ok(Box::new(move || {
            let page_xml = Page::image_template(page_number, image_path);
            let image: EpubContent<&[u8]> = EpubContent::new(page_path, page_xml.as_ref())
                .title(title)
                .reftype(reference_type);

            let mut builder = builder.lock().unwrap();

            (*builder).add_content(image).unwrap();

            page_number
        }))
    }
}

impl RemotePage {
    pub async fn into_async(&self, token: &String) -> anyhow::Result<(usize, Page)> {
        let page_number = self.page_number - 1;
        let url = format!(
            "https://api.kodansha.us/comic/{volume}/pages/{page}",
            volume = self.comic_id,
            page = page_number
        );

        let page = reqwest::Client::new()
            .get(url)
            .header("authorization", format!("Bearer {}", token))
            .send()
            .await?
            .json::<Page>()
            .await?;

        Ok((page_number, page))
    }
}
