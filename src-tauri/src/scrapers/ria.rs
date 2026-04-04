use super::{NewsScraper, Article};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tokio::sync::Mutex;
use rand::seq::SliceRandom;

pub struct RiaScraper {
    url_pool: Mutex<Vec<String>>,
}

impl RiaScraper {
    pub fn new() -> Self {
        Self {
            url_pool: Mutex::new(Vec::new()),
        }
    }

    async fn ensure_pool_filled(&self, client: &Client) -> Result<(), String> {
        let mut pool = self.url_pool.lock().await;
        if !pool.is_empty() {
            return Ok(());
        }

        let index_url = "https://ria.ru/sitemap_article_index.xml";
        let index_text = client
            .get(index_url).send().await.map_err(|e| e.to_string())?
            .text().await.map_err(|e| e.to_string())?;

        let monthly_sitemaps: Vec<String> = {
            let doc = Html::parse_document(&index_text);
            let sel = Selector::parse("sitemapindex > sitemap > loc").unwrap();
            doc.select(&sel)
                .filter_map(|el| el.text().next().map(|t| t.trim().to_string()))
                .take(2).collect()
        };

        let loc_sel = Selector::parse("urlset > url > loc").unwrap();
        for sitemap_url in &monthly_sitemaps {
            if let Ok(resp) = client.get(sitemap_url).send().await {
                if let Ok(body) = resp.text().await {
                    let doc = Html::parse_document(&body);
                    for el in doc.select(&loc_sel) {
                        if let Some(url) = el.text().next() {
                            let url = url.trim().to_string();
                            if url.ends_with(".html") {
                                pool.push(url);
                            }
                        }
                    }
                }
            }
        }
        pool.dedup();
        Ok(())
    }

    async fn scrape_article(&self, client: &Client, url: &str) -> Result<Article, String> {
        let body = client
            .get(url).send().await.map_err(|e| e.to_string())?
            .text().await.map_err(|e| e.to_string())?;
        let doc = Html::parse_document(&body);

        let title = doc.select(&Selector::parse("div.article__title").unwrap())
            .next().map(|el| el.text().collect::<String>().trim().to_string()).unwrap_or_default();

        let cover = doc.select(&Selector::parse(r#"meta[name="relap-image"]"#).unwrap())
            .next().and_then(|el| el.value().attr("content").map(|c| c.to_string()))
            .or_else(|| doc.select(&Selector::parse(r#"meta[property="og:image"]"#).unwrap())
                .next().and_then(|el| el.value().attr("content").map(|c| c.to_string())))
            .unwrap_or_default();

        let content_sel = Selector::parse("div.article__text").unwrap();
        let mut content = String::new();

        if let Some(sub) = doc.select(&Selector::parse("h1.article__second-title").unwrap()).next() {
            content.push_str(&sub.text().collect::<String>());
            content.push('\n');
        }

        for el in doc.select(&content_sel) {
            content.push_str(&el.text().collect::<String>());
            content.push('\n');
        }

        if title.is_empty() || content.trim().is_empty() {
            return Err("Failed to parse".into());
        }

        Ok(Article {
            source_id: self.id().to_string(),
            source_name: self.name().to_string(),
            language: self.language().to_string(),
            url: url.to_string(),
            title,
            cover_image: cover,
            content: content.trim().to_string(),
            words: None,
            difficulty: None,
            recommendation: None,
        })
    }
}

#[async_trait]
impl NewsScraper for RiaScraper {
    fn language(&self) -> &str { "ru" }
    fn id(&self) -> &str { "ria_ru" }
    fn name(&self) -> &str { "РИА Новости" }

    async fn fetch_articles(
        &self,
        client: &Client,
        limit: usize,
        excluded_urls: &HashSet<String>,
    ) -> Result<Vec<Article>, String> {
        self.ensure_pool_filled(client).await?;
        let pool = self.url_pool.lock().await;

        let available_urls: Vec<&String> = pool
            .iter()
            .filter(|url| !excluded_urls.contains(*url))
            .collect();

        if available_urls.is_empty() {
            return Ok(Vec::new());
        }

        let actual_limit = limit.min(available_urls.len());
        let chosen_urls: Vec<&String> = {
            let mut rng = rand::thread_rng();
            available_urls
            .choose_multiple(&mut rng, actual_limit)
            .map(|s| *s)
            .collect()
        };

        let mut articles = Vec::with_capacity(actual_limit);
        for url in chosen_urls {
            match self.scrape_article(client, url).await {
                Ok(a) => articles.push(a),
                Err(_) => continue,
            }
        }

        Ok(articles)
    }

}
