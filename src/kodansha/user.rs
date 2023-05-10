use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Ok};
use reqwest;
use serde::{Deserialize, Serialize};
use tokio::fs::{create_dir, File};
use tokio::io::copy;

use crate::Volume;

use super::{library, Library};

const CONFIG_DIR: &str = "k-download";
const CONFIG_FILE: &str = "config.toml";
const TOKEN_FILE: &str = "token";

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
}

#[derive(Clone)]
pub struct User {
    pub token: String,
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
        let token = self.token.clone();

        copy(&mut token.as_bytes(), &mut file).await?;

        Ok(())
    }

    pub fn library(&self) -> Arc<Mutex<Option<Library>>> {
        self.library.clone()
    }

    pub async fn load_library(&mut self) -> anyhow::Result<()> {
        match self.library.lock() {
            Result::Ok(mut library) => {
                *library = Some(Library {
                    volumes: reqwest::Client::new()
                        .get("https://api.kodansha.us/mycomics/")
                        .header("authorization", format!("Bearer {}", self.token.clone()))
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
                            }),

                            None => None,
                        })
                        .collect(),
                });

                Ok(())
            }
            Err(err) => Result::Err(anyhow!("Couldn't read the mutex")),
        }
    }
}

impl From<KodanshaUser> for User {
    fn from(value: KodanshaUser) -> Self {
        Self {
            token: value.token,
            library: Arc::default(),
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

            User {
                token,
                library: Arc::default(),
            }
        } else {
            let user = User::new(self.username, self.password).await;

            user.persist(token_path).await?;

            user
        };

        Ok(user)
    }
}
