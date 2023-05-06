use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Credentials {
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
}
