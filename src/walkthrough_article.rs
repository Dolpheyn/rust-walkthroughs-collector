use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub type WalkthroughArticlesByIssueLink = HashMap<String, Vec<WalkthroughArticle>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct WalkthroughArticle {
    pub title: String,
    pub link: String,
}
