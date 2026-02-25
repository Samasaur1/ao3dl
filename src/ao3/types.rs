use serde::Deserialize;

#[derive(Deserialize)]
pub struct AuthenticityToken {
    pub token: String,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(untagged)]
pub enum WorkId {
    Bare(usize),
    WithTimestamp { id: usize, timestamp: usize },
}

impl WorkId {
    pub fn id(&self) -> &usize {
        match self {
            WorkId::Bare(id) => id,
            WorkId::WithTimestamp { id, timestamp: _ } => id,
        }
    }
}
