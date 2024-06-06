use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct WalkthroughArticle {
    pub title: String,
    pub link: String,
}
