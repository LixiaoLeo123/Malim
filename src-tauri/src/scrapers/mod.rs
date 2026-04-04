/*
===================================================================================================
|  This project is solely for learning Rust language and studying web scraping techniques.        |
|  The code does not contain any data by default,                                                 |
|  and users must comply with the target website's robots.txt and relevant laws and regulations.  |
|  It is strictly prohibited for commercial use or large-scale data scraping.                     |
|  The author is not responsible for any legal consequences arising from the misuse of the code.  |
===================================================================================================
*/


pub mod ria;
pub mod kp;
pub mod registry;
pub mod commands;
pub mod yna;
pub mod kbs;
pub mod tmt;
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
    pub difficulty: Option<f64>,
    pub recommendation: Option<f64>,
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
