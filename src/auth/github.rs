use super::{
    users::{User, UserMigration},
    CallbackHandler,
};

pub struct GitHubLogin(pub String);

#[derive(Debug, serde::Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub login: String,
    pub name: String,
    pub email: String,
}

impl UserMigration for UserInfo {
    fn migrate(self, default_language: &str) -> User {
        User::temporary(self.email, self.name, default_language)
    }
}

impl CallbackHandler for GitHubLogin {
    type IntermediateUserInfo = UserInfo;

    const SERVICE_NAME: &'static str = "GitHub";

    fn get_request_client(&self) -> reqwest::RequestBuilder {
        reqwest::Client::new()
            .get("https://api.github.com/user")
            .header("User-Agent", "Test-App")
            .header(
                "Accept",
                "application/vnd.github+json,application/vnd.github.diff",
            )
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("Authorization", format!("Bearer {}", self.0))
    }
}
