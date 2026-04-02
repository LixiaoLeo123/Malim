pub mod ria;
pub mod registry;

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SourceInfo {
    pub id: String,
    pub name: String,
    pub language: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Article {
    pub source_id: String,
    pub source_name: String,
    pub language: String,
    pub url: String,
    pub title: String,
    pub cover_image: String,
    pub content: String,

    pub words: Option<Vec<(String, Option<f64>)>>,
}


#[async_trait]
pub trait NewsScraper: Send + Sync {
    fn language(&self) -> &str;
    fn id(&self) -> &str;
    fn name(&self) -> &str;

    async fn fetch_articles(
        &self,
        client: &Client,
        limit: usize,
        excluded_urls: &HashSet<String>,
    ) -> Result<Vec<Article>, String>;
}
