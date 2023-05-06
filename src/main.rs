use std::io;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use epub_builder::EpubBuilder;
use epub_builder::EpubVersion;
use epub_builder::ZipLibrary;
use futures::future::join_all;
use kodansha_downloader::User;
use kodansha_downloader::Volume;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(about = "Downloads a volume from Kodansha's web reader")]
struct Cli {
    #[structopt(short, long)]
    username: String,
    #[structopt(short, long)]
    password: String,
    #[structopt(
        short,
        long,
        help = "The volume number in the url (e.g. \"https://api.kodansha.us/comic/10\" is 10)"
    )]
    volume: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::from_args();

    let volume = Volume::get(cli.volume).await?;

    let user = User::new(cli.username.into(), cli.password.into()).await;

    let zip = ZipLibrary::new().unwrap();
    let mut builder = EpubBuilder::new(zip).unwrap();

    builder
        .metadata("title", volume.volume_name.clone())
        .unwrap()
        .metadata(
            "description",
            volume.description.clone().replace("rsquo", "apos"),
        )
        //.unwrap()
        //.metadata("series", volume.series_name.clone())
        .unwrap()
        .metadata("subject", "Manga")
        .unwrap()
        .epub_version(EpubVersion::V30);

    let builder = Arc::new(Mutex::new(builder));

    join_all(volume.page_links(&user).await.map(|(page_number, page)| {
        let user = user.clone();
        let builder = builder.clone();
        tokio::spawn(async move {
            page.write_to_epub(page_number, builder, &user)
                .await
                .unwrap()
        })
    }))
    .await;

    let mut builder = builder.lock().unwrap();
    let _ = builder.generate(&mut io::stdout());

    Ok(())
}
