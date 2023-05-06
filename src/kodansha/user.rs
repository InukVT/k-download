use anyhow::anyhow;
use config::Config;
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};

use super::Library;

const CONFIG_DIR: &str = ".k-download";
const CONFIG_FILE: &str = "config.toml";
const TOKEN_FILE: &str = "token";

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    #[serde(alias = "UserName")]
    username: String,
    #[serde(alias = "Password")]
    password: String,
}

#[derive(Deserialize, Clone)]
pub struct User {
    #[serde(alias = "access_token")]
    pub token: String,
}

impl User {
    pub async fn new(username: String, password: String) -> User {
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

    async fn persist(&self) -> anyhow::Result<()> {
        let mut data_dir = dirs::data_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);
        data_dir.push(TOKEN_FILE);

        let token_file = data_dir.into_os_string();

        let file = File::create(token_file).await?;
        let mut writer = BufWriter::new(file);
        let token = self.token.clone();
        let buffer = token.as_bytes();
        writer.write_all(buffer).await?;

        Ok(())
    }

    async fn library(&self) -> anyhow::Result<Library> {
        let library = reqwest::Client::new()
            .get("https://api.kodansha.us/mycomics/")
            .header("authorization", self.token.clone())
            .send()
            .await?
            .json::<Library>()
            .await?;

        Ok(library)
    }
}

impl Credentials {
    pub fn from_config() -> anyhow::Result<Credentials> {
        let mut data_dir = dirs::config_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);
        data_dir.push(CONFIG_FILE);

        let config_file = data_dir.into_os_string();
        let config_str = config_file
            .to_str()
            .ok_or(anyhow!("Error converting dir to str"))?;

        let config = Config::builder()
            .add_source(config::File::with_name(config_str))
            .build()?
            .try_deserialize::<Credentials>()?;

        return Ok(config);
    }

    pub async fn login(self) -> anyhow::Result<User> {
        let mut data_dir = dirs::data_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);
        data_dir.push(TOKEN_FILE);

        let token_file = data_dir.into_os_string();

        match File::open(token_file).await {
            Ok(mut file) => {
                let mut token = String::new();
                file.read_to_string(&mut token).await?;
                Ok(User { token })
            }
            Err(_) => {
                let user = User::new(self.username, self.password).await;

                user.persist().await?;

                Ok(user)
            }
        }
    }
}
