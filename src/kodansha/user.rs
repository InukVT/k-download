use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};

use anyhow::{anyhow, Ok};
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::{
    fs::{create_dir, File},
    io::copy,
};

use crate::Volume;

use super::Library;

const CONFIG_DIR: &str = "k-download";
const CONFIG_FILE: &str = "config.toml";
const TOKEN_FILE: &str = "token.toml";

#[derive(Serialize, Deserialize, Debug)]
pub struct Credentials {
    #[serde(alias = "UserName")]
    pub username: String,
    #[serde(alias = "Password")]
    password: String,
}

#[derive(Deserialize)]
struct KodanshaUser {
    #[serde(alias = "access_token")]
    pub token: String,
    #[serde(alias = "refresh_token")]
    pub refresh: String,
    #[serde(with = "from_str")]
    pub expires_in: i64,
}

#[derive(Deserialize)]
struct KodanshaRefresh {
    pub access_token: String,
    #[serde(with = "from_str")]
    pub expires_in: i64,
}

#[derive(Serialize)]
struct KodanshaRefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize)]
struct StoredUser {
    token: String,
    refresh: String,
    expirery: DateTime<Utc>,
}

#[derive(Clone)]
pub struct User {
    token: String,
    refresh: String,
    expirery: DateTime<Utc>,
    library: Arc<Mutex<Option<Library>>>,
}

impl User {
    pub async fn new(username: String, password: String) -> User {
        let creds = Credentials { username, password };

        reqwest::Client::new()
            .post("https://api.kodansha.us/account/token")
            .json(&creds)
            .send()
            .await
            .unwrap()
            .json::<KodanshaUser>()
            .await
            .unwrap()
            .into()
    }

    async fn persist(&self, path: &str) -> anyhow::Result<()> {
        let mut file = File::create(path).await?;
        let stored: StoredUser = self.into();

        let data = toml::to_string_pretty(&stored)?;

        copy(&mut data.as_bytes(), &mut file).await?;

        Ok(())
    }

    pub fn library(&self) -> Arc<Mutex<Option<Library>>> {
        self.library.clone()
    }

    pub async fn token(&mut self) -> anyhow::Result<String> {
        let mut data_dir = dirs::data_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);
        data_dir.push(TOKEN_FILE);

        let token_file = data_dir.into_os_string();
        let token_path = token_file
            .to_str()
            .ok_or(anyhow!("Couldn't convert options path to path"))?;

        let now = Utc::now();
        if dbg!(now.timestamp()) < dbg!(self.expirery.timestamp()) {
            return Ok(self.token.clone());
        }

        let refresh: KodanshaRefreshRequest = self.into();
        let request = reqwest::Client::new()
            .post("https://api.kodansha.us/account/token")
            .json(&refresh)
            .send()
            .await?;

        let refresh = request.json::<KodanshaRefresh>().await?;

        self.token = refresh.access_token;
        self.expirery = expirery(refresh.expires_in);
        self.persist(token_path).await?;

        Ok(self.token.clone())
    }

    pub async fn load_library(&mut self) -> anyhow::Result<()> {
        // Prefetch so we don't indefinetly hold the mutex in an async context
        let token = self.token().await?;
        let fetched_library = Library {
            volumes: reqwest::Client::new()
                .get("https://api.kodansha.us/mycomics/")
                .header("authorization", format!("Bearer {}", token))
                .send()
                .await?
                .json::<Vec<Volume<Option<String>>>>()
                .await?
                .into_iter()
                // Filters away chapters
                .filter_map(|volume| match volume.volume_name {
                    Some(volume_name) => Some(Volume {
                        series_name: volume.series_name,
                        volume_name,
                        volume_number: volume.volume_number,
                        page_count: volume.page_count,
                        description: volume.description,
                        id: volume.id,
                        series_id: volume.series_id,
                    }),

                    None => None,
                })
                .collect(),
        };

        let library = { self.library.lock() };
        match library {
            Result::Ok(mut library) => {
                *library = Some(fetched_library);

                Ok(())
            }
            Err(_) => Result::Err(anyhow!("Couldn't read the mutex")),
        }
    }
}

impl Credentials {
    pub fn new(username: String, password: String) -> Credentials {
        Credentials { username, password }
    }
    pub async fn from_config() -> anyhow::Result<Credentials> {
        let mut data_dir = dirs::config_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);

        let option_dir = data_dir.clone().into_os_string();
        let option_str = option_dir
            .to_str()
            .ok_or(anyhow!("Couldn't convert options path to path"))?;
        if !Path::new(option_str).exists() {
            return Err(anyhow!("File doesn't exist"));
        }

        data_dir.push(CONFIG_FILE);

        let config_file = data_dir.into_os_string();

        let config_dir = config_file
            .to_str()
            .ok_or(anyhow!("Error converting dir to str"))?;

        let creds = match Path::new(config_dir).exists() {
            true => {
                let contest = tokio::fs::read_to_string(config_dir).await?;

                toml::from_str::<Credentials>(&contest)?
            }
            false => {
                return Err(anyhow!(
                    "Couldn't pass the file at {}, pleaser consider deleting it or edit it.",
                    config_dir
                ))
            }
        };

        Ok(creds)
    }

    pub async fn write_user(username: String, password: String) -> anyhow::Result<Credentials> {
        let mut data_dir = dirs::config_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);

        let option_dir = data_dir.clone().into_os_string();
        let option_str = option_dir
            .to_str()
            .ok_or(anyhow!("Couldn't convert options path to path"))?;
        if !Path::new(option_str).exists() {
            create_dir(option_str).await?;
        }

        data_dir.push(CONFIG_FILE);

        let config_file = data_dir.into_os_string();

        let config_str = config_file
            .to_str()
            .ok_or(anyhow!("Error converting dir to str"))?;
        let creds = Credentials { username, password };
        let mut file = File::create(config_str).await?;

        let data = toml::to_string_pretty(&creds)?;

        copy(&mut data.as_bytes(), &mut file).await?;
        Ok(creds)
    }

    pub async fn login(self) -> anyhow::Result<User> {
        let mut data_dir = dirs::data_dir().ok_or(anyhow!("No data dir"))?;
        data_dir.push(CONFIG_DIR);
        data_dir.push(TOKEN_FILE);

        let token_file = data_dir.into_os_string();
        let token_path = token_file
            .to_str()
            .ok_or(anyhow!("Couldn't convert options path to path"))?;

        let user = if Path::new(token_path).exists() {
            let token = tokio::fs::read_to_string(token_path).await?;
            let stored = toml::from_str::<StoredUser>(&token)?;

            stored.into()
        } else {
            let user = User::new(self.username, self.password).await;

            user.persist(token_path).await?;

            user
        };

        Ok(user)
    }
}

impl From<KodanshaUser> for User {
    fn from(value: KodanshaUser) -> Self {
        let expirery = expirery(value.expires_in);
        Self {
            token: value.token,
            library: Arc::default(),
            expirery,
            refresh: value.refresh,
        }
    }
}

impl From<&StoredUser> for User {
    fn from(value: &StoredUser) -> Self {
        Self {
            token: value.token.clone(),
            refresh: value.refresh.clone(),
            expirery: value.expirery,
            library: Arc::default(),
        }
    }
}

impl From<StoredUser> for User {
    fn from(value: StoredUser) -> Self {
        Self {
            token: value.token,
            refresh: value.refresh,
            expirery: value.expirery,
            library: Arc::default(),
        }
    }
}

impl From<&User> for StoredUser {
    fn from(value: &User) -> Self {
        Self {
            token: value.token.clone(),
            refresh: value.refresh.clone(),
            expirery: value.expirery,
        }
    }
}

impl From<&mut User> for KodanshaRefreshRequest {
    fn from(value: &mut User) -> Self {
        Self {
            refresh_token: value.refresh.clone(),
        }
    }
}

fn expirery(expires_in: i64) -> DateTime<Utc> {
    let now = Utc::now();
    let offset = Duration::seconds(expires_in);
    // give expiration a slack case of super slow connections
    let slack = Duration::minutes(60);

    now + offset - slack
}

mod from_str {
    use serde::{self, Deserialize, Deserializer};

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        let str: &str = string.as_str();
        i64::from_str_radix(str, 10).map_err(serde::de::Error::custom)
    }
}
