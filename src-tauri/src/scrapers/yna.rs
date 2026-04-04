use super::{NewsScraper, Article};
use async_trait::async_trait;
use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashSet;
use tokio::sync::Mutex;
use rand::seq::SliceRandom;

pub struct YnaScraper {
    url_pool: Mutex<Vec<String>>,
}

impl YnaScraper {
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

        let sitemap_url = "https://www.yna.co.kr/news-sitemap3.xml";
        let sitemap_text = client
            .get(sitemap_url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&sitemap_text);
        let loc_sel = Selector::parse("urlset > url > loc").unwrap();
        let title_sel = Selector::parse("url > news\\:news > news\\:title").unwrap();
        let mut entries: Vec<(String, String)> = Vec::new();

        let mut loc_iter = doc.select(&loc_sel);
        let mut title_iter = doc.select(&title_sel);

        loop {
            let loc_opt = loc_iter.next();
            let title_opt = title_iter.next();

            if loc_opt.is_none() || title_opt.is_none() {
                break;
            }

            let loc_el = loc_opt.unwrap();
            let title_el = title_opt.unwrap();

            let url = loc_el
                .text()
                .next()
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            let title = title_el
                .text()
                .next()
                .map(|t| t.trim().to_string())
                .unwrap_or_default();

            if !url.is_empty() {
                entries.push((url, title));
            }
        }

        for (url, _title) in entries {
            pool.push(url);
        }

        pool.dedup();
        Ok(())
    }

    async fn scrape_article(
        &self,
        client: &Client,
        url: &str,
        sitemap_title: Option<&str>,
    ) -> Result<Article, String> {
        let body = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .text()
            .await
            .map_err(|e| e.to_string())?;

        let doc = Html::parse_document(&body);

        let title = if let Some(t) = sitemap_title {
            t.to_string()
        } else {
            let og_title = doc
                .select(&Selector::parse(r#"meta[property="og:title"]"#).unwrap())
                .next()
                .and_then(|el| el.value().attr("content").map(|c| c.trim().to_string()))
                .unwrap_or_default();

            if !og_title.is_empty() {
                og_title
            } else {
                doc.select(&Selector::parse("title").unwrap())
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_default()
            }
        };

        let cover = doc
            .select(&Selector::parse(r#"meta[property="og:image"]"#).unwrap())
            .next()
            .and_then(|el| el.value().attr("content").map(|c| c.to_string()))
            .or_else(|| {
                doc.select(&Selector::parse(r#"img[data-pop-open="pop-photo"]"#).unwrap())
                    .next()
                    .and_then(|el| el.value().attr("src").map(|c| c.to_string()))
            })
            .unwrap_or_default();

        let p_sel = Selector::parse("p").unwrap();
        let mut content = String::new();

        for el in doc.select(&p_sel) {
            let text = el.text().collect::<String>();
            let text = text.trim();
            if !text.is_empty() {
                content.push_str(text);
                content.push('\n');
            }
        }

        if content.trim().is_empty() {
            let view_sel = Selector::parse("div.article-view").unwrap();
            for el in doc.select(&view_sel) {
                let text = el.text().collect::<String>();
                let text = text.trim();
                if !text.is_empty() {
                    content.push_str(text);
                    content.push('\n');
                }
            }
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
impl NewsScraper for YnaScraper {
    fn language(&self) -> &str {
        "ko"
    }

    fn id(&self) -> &str {
        "yna_kr"
    }

    fn name(&self) -> &str {
        "연합뉴스"
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
            match self.scrape_article(client, url, None).await {
                Ok(a) => articles.push(a),
                Err(_) => continue,
            }
        }

        Ok(articles)
    }
}
