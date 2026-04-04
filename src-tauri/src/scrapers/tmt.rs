use super::{Article, NewsScraper};
use async_trait::async_trait;
use rand::seq::SliceRandom;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tokio::sync::Mutex;
use regex::Regex;
pub struct TmtScraper {
    url_pool: Mutex<Vec<String>>,
}

impl TmtScraper {
    pub fn new() -> Self {
        Self {
            url_pool: Mutex::new(Vec::new()),
        }
    }

    async fn ensure_pool_filled(&self, client: &Client) -> Result<(), String> {
        let mut pool = self.url_pool.lock().await;
        dbg!(&pool);
        if !pool.is_empty() {
            return Ok(());
        }

        let sitemap_index_url = "https://ru.themoscowtimes.com/sitemap/sitemap.xml";
        let index_text = client
            .get(sitemap_index_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let first_sitemap_url = {
            let re = Regex::new(r"<loc>\s*(https?://[^<]+?)\s*</loc>").unwrap();

            let url = re.captures_iter(index_text.as_str())
                .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
                .next()
                .ok_or("No sitemap URL found in index")?;
            url
        };

        dbg!(&first_sitemap_url);
        let sitemap_text = client
            .get(&*first_sitemap_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        pool.extend(
            {
                let re = Regex::new(r"<loc>\s*(https?://[^<]+?)\s*</loc>").unwrap();

                re.captures_iter(sitemap_text.as_str())
                    .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
                    // .map(|link| link.replace("https://www.themoscowtimes.com/", "https://ru.themoscowtimes.com/"))
                    .collect::<Vec<String>>()

            }
        );

        pool.dedup();
        Ok(())
    }

    async fn scrape_article(&self, client: &Client, url: &str) -> Result<Article, String> {
        let body = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&body);

        let title = doc
            .select(&Selector::parse("title").unwrap())
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .and_then(|t| {
                let t = t.trim();
                if t.ends_with(" - The Moscow Times") {
                    Some(t.trim_end_matches(" - The Moscow Times").to_string())
                } else {
                    Some(t.to_string())
                }
            })
            .unwrap_or_default();

        let cover = doc
            .select(
                &Selector::parse(
                    r#"img[src^="https://static.themoscowtimes.com/image/article_1360"]"#,
                )
                .unwrap(),
            )
            .next()
            .and_then(|el| el.value().attr("src").map(|s| s.to_string()))
            .or_else(|| {
                doc.select(&Selector::parse(r#"meta[property="og:image"]"#).unwrap())
                    .next()
                    .and_then(|el| el.value().attr("content").map(|s| s.to_string()))
            })
            .unwrap_or_default();

        let p_sel = Selector::parse("p").unwrap();
        let mut content = String::new();

        for p in doc.select(&p_sel) {
            let text = p.text().collect::<String>();
            let text = text.trim();
            if text.is_empty() {
                continue;
            }
            content.push_str(text);
            content.push('\n');
        }

        if title.is_empty() || content.trim().is_empty() {
            return Err("Failed to parse: title or content is empty".into());
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
impl NewsScraper for TmtScraper {
    fn language(&self) -> &str {
        "ru"
    }

    fn id(&self) -> &str {
        "tmt_ru"
    }

    fn name(&self) -> &str {
        "The Moscow Times"
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
