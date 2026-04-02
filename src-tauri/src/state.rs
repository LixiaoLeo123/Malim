// src/state.rs
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use crate::scrapers::{NewsScraper, SourceInfo};

pub struct AppState {
    pub http_client: reqwest::Client,

    pub scrapers_by_lang: HashMap<String, Vec<Box<dyn NewsScraper>>>,

    pub emitted_urls: Mutex<HashSet<String>>,
}

impl AppState {
    pub fn get_sources_for_lang(&self, lang: &str) -> Vec<SourceInfo> {
        match self.scrapers_by_lang.get(lang) {
            Some(scrapers) => scrapers
                .iter()
                .map(|s| SourceInfo {
                    id: s.id().to_string(),
                    name: s.name().to_string(),
                    language: s.language().to_string(),
                })
                .collect(),
            None => vec![],
        }
    }
}
