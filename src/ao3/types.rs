use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthenticityToken {
    pub token: String,
}
