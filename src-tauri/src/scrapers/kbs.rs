use super::{NewsScraper, Article};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tokio::sync::Mutex;
use rand::seq::SliceRandom;

pub struct KbsScraper {
    url_pool: Mutex<Vec<String>>,
}

#[derive(Debug, Clone)]
struct SitemapItem {
    url: String,
    title: String,
}

impl KbsScraper {
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

        let sitemap_url = "https://news.kbs.co.kr/sitemap/recentNewsList.xml";
        let sitemap_text = client
            .get(sitemap_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&sitemap_text);

        let url_sel = Selector::parse("url").unwrap();

        let loc_sel = Selector::parse("loc").unwrap();

        for url_node in doc.select(&url_sel) {
            let url = url_node
                .select(&loc_sel)
                .next()
                .and_then(|el| el.text().next())
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            if !url.is_empty() {
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
        let sitemap_url = "https://news.kbs.co.kr/sitemap/recentNewsList.xml";
        let sitemap_text = client
            .get(sitemap_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&sitemap_text);

        let url_sel = Selector::parse("url").unwrap();
        let loc_sel = Selector::parse("loc").unwrap();
        let title_sel = Selector::parse("news\\:news > news\\:title").unwrap();

        let mut items = Vec::with_capacity(urls.len());

        for url_node in doc.select(&url_sel) {
            let loc = url_node
                .select(&loc_sel)
                .next()
                .and_then(|el| el.text().next())
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            if !urls.contains(&loc) {
                continue;
            }

            let title = url_node
                .select(&title_sel)
                .next()
                .and_then(|el| el.text().next())
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            items.push(SitemapItem {
                url: loc,
                title,
            });
        }

        Ok(items)
    }

    async fn scrape_article(
        &self,
        client: &Client,
        item: &SitemapItem,
    ) -> Result<Article, String> {
        let SitemapItem { url, title } = item;

        let body = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let article_doc = Html::parse_document(&body);
        let content = {
            let cont_sel = Selector::parse("#cont_newstext").unwrap();
            let body_sel = Selector::parse(".detail-body").unwrap();

            let mut text = String::new();

            if let Some(el) = article_doc.select(&cont_sel).next() {
                text.push_str(&el.text().collect::<String>());
            } else if let Some(el) = article_doc.select(&body_sel).next() {
                text.push_str(&el.text().collect::<String>());
            }

            text.trim().to_string()
        };

        let title = if title.is_empty() {
            article_doc
                .select(&Selector::parse("title").unwrap())
                .next()
                .map(|el| el.text().collect::<String>().trim().to_string())
                .unwrap_or_default()
        } else {
            title.clone()
        };

        let cover = article_doc
            .select(&Selector::parse("#imgVodThumbnail").unwrap())
            .next()
            .and_then(|el| el.value().attr("src").map(|s| s.to_string()))
            .or_else(|| {
                article_doc
                    .select(&Selector::parse(r#"meta[property="og:image"]"#).unwrap())
                    .next()
                    .and_then(|el| el.value().attr("content").map(|s| s.to_string()))
            })
            .unwrap_or_default();
        let cover = if cover.starts_with("//") {
            format!("https:{}", cover)
        } else {
            cover
        };

        if title.is_empty() || content.is_empty() {
            return Err("Failed to parse: title or content is empty".into());
        }

        Ok(Article {
            source_id: self.id().to_string(),
            source_name: self.name().to_string(),
            language: self.language().to_string(),
            url: url.clone(),
            title,
            cover_image: cover,
            content,
            words: None,
            difficulty: None,
            recommendation: None,
        })
    }
}

#[async_trait]
impl NewsScraper for KbsScraper {
    fn language(&self) -> &str {
        "kr"
    }

    fn id(&self) -> &str {
        "kbs_kr"
    }

    fn name(&self) -> &str {
        "KBS뉴스"
    }

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
