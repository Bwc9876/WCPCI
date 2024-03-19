use super::{
    users::{User, UserMigration},
    CallbackHandler,
};

pub struct GoogleLogin(pub String);

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Name {
    pub given_name: String,
    pub family_name: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailAddress {
    pub value: String,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    pub email_addresses: Vec<EmailAddress>,
    pub names: Vec<Name>,
}

impl UserMigration for UserInfo {
    fn migrate(self, default_language: &str) -> User {
        let email = self.email_addresses.first().unwrap().value.clone();
        let name = self
            .names
            .first()
            .map(|n| format!("{} {}", n.given_name, n.family_name))
            .unwrap_or_else(|| email.clone());
        User::temporary(email, name, default_language)
    }
}

impl CallbackHandler for GoogleLogin {
    type IntermediateUserInfo = UserInfo;

    const SERVICE_NAME: &'static str = "Google";

    fn get_request_client(&self) -> reqwest::RequestBuilder {
        reqwest::Client::new()
            .get("https://people.googleapis.com/v1/people/me?personFields=names,emailAddresses")
            .header("User-Agent", "Test-App")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.0))
    }
}
