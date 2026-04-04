use super::{NewsScraper, Article};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tokio::sync::Mutex;
use rand::seq::SliceRandom;

pub struct KpScraper {
    url_pool: Mutex<Vec<String>>,
}

#[derive(Debug, Clone)]
struct SitemapItem {
    url: String,
    title: String,
    cover: String,
}

impl KpScraper {
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

        let sitemap_url = "https://www.kp.ru/sitemap/main_01.xml";
        let sitemap_text = client
            .get(sitemap_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&sitemap_text);

        let loc_sel = Selector::parse("url > loc").unwrap();
        for el in doc.select(&loc_sel) {
            if let Some(url) = el.text().next() {
                let url = url.trim().to_string();
                pool.push(url);
            }
        }
        pool.dedup();
        Ok(())
    }

    async fn extract_sitemap_items(
        &self,
        client: &Client,
        urls: &[String],
    ) -> Result<Vec<SitemapItem>, String> {
        let sitemap_url = "https://www.kp.ru/sitemap/main_01.xml";
        let sitemap_text = client
            .get(sitemap_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&sitemap_text);

        let mut items = Vec::with_capacity(urls.len());

        for url in urls {
            let title = {
                let title_sel = Selector::parse("url > title").unwrap();
                doc.select(&title_sel)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_default()
            };

            let cover = {
                let image_sel = Selector::parse("url > image").unwrap();
                doc.select(&image_sel)
                    .next()
                    .and_then(|el| {
                        let loc_sel = Selector::parse("loc").unwrap();
                        el.select(&loc_sel)
                            .next()
                            .and_then(|loc| loc.text().next().map(|t| t.trim().to_string()))
                    })
                    .unwrap_or_default()
            };

            items.push(SitemapItem {
                url: url.clone(),
                title,
                cover,
            });
        }

        Ok(items)
    }

    async fn scrape_article(
        &self,
        client: &Client,
        item: &SitemapItem,
    ) -> Result<Article, String> {
        let SitemapItem { url, title, cover } = item;

        let body = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let article_doc = Html::parse_document(&body);

        let p_sel = Selector::parse("p").unwrap();
        let mut content = String::new();
        for p in article_doc.select(&p_sel) {
            let text = p.text().collect::<String>();
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            content.push_str(text);
            content.push('\n');
        }

        let title = if title.is_empty() {
            article_doc
                .select(&Selector::parse("title").unwrap())
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default()
        } else {
            title.clone()
        };

        let cover = if cover.is_empty() {
            article_doc
                .select(&Selector::parse(r#"meta[property="og:image"]"#).unwrap())
                .next()
                .and_then(|el| el.value().attr("content").map(|c| c.to_string()))
                .unwrap_or_default()
        } else {
            cover.clone()
        };

        if title.is_empty() || content.trim().is_empty() {
            return Err("Failed to parse: title or content is empty".into());
        }

        Ok(Article {
            source_id: self.id().to_string(),
            source_name: self.name().to_string(),
            language: self.language().to_string(),
            url: url.clone(),
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
impl NewsScraper for KpScraper {
    fn language(&self) -> &str { "ru" }
    fn id(&self) -> &str { "kp_ru" }
    fn name(&self) -> &str { "Комсомольская правда" }

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

        let sitemap_items = self
            .extract_sitemap_items(client, &chosen_urls.into_iter().cloned().collect::<Vec<_>>())
            .await?;

        let mut articles = Vec::with_capacity(sitemap_items.len());
        for item in &sitemap_items {
            match self.scrape_article(client, item).await {
                Ok(a) => articles.push(a),
                Err(_) => continue,
            }
        }

        Ok(articles)
    }
}
