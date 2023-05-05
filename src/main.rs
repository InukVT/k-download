use futures::future::join_all;
use reqwest;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Volume {
    series_name: String,
    volume_name: String,
    volume_number: u8,
    page_count: u16,

    url_id: Option<u16>,
}

#[derive(Deserialize)]
struct Page {
    url: String,
}

#[derive(Deserialize)]
struct User {
    #[serde(alias = "access_token")]
    token: String,
}

#[derive(Serialize)]
struct Credentials {
    #[serde(alias = "UserName")]
    username: String,
    #[serde(alias = "Password")]
    password: String,
}

impl User {
    async fn new(username: String, password: String) -> User {
        let mut creds = Credentials { username, password };
        reqwest::Client::new()
            .post("https://api.kodansha.us/account/token")
            .json(&mut creds)
            .send()
            .await
            .unwrap()
            .json::<User>()
            .await
            .unwrap()
    }
}

impl Volume {
    async fn get(url_id: u16) -> Result<Volume, reqwest::Error> {
        let volume_route = format!("https://api.kodansha.us/comic/{}/", url_id);
        let mut volume = reqwest::get(volume_route).await?.json::<Volume>().await?;
        volume.url_id = Some(url_id);

        Ok(volume)
    }

    async fn page_links(self, user: &User) -> Vec<Page> {
        let volume_route = format!("https://api.kodansha.us/comic/{}", self.url_id.unwrap());

        join_all((0..self.page_count).map(|index| {
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
        .collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let cli = Cli::from_args();

    let volume = Volume::get(cli.volume).await?;

    let user = User::new(cli.username.into(), cli.password.into()).await;
    for page in volume.page_links(&user).await {
        println!("{}", page.url);
    }

    Ok(())
}
