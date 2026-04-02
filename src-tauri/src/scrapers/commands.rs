use crate::article::{Article, SourceInfo};
use crate::memory;
use crate::state::AppState;
use serde::Deserialize;
use tauri::{AppHandle, State};

#[derive(Deserialize)]
pub struct GetFeedRequest {
    pub lang: String,
    pub selected_source_ids: Vec<String>,
    pub limit: usize,
    pub mark_familiarity: bool,
}

#[tauri::command]
pub fn get_sources_by_language(state: State<'_, AppState>, lang: String) -> Vec<SourceInfo> {
    state.get_sources_for_lang(&lang)
}

#[tauri::command]
pub async fn get_feed(
    app: AppHandle,
    state: State<'_, AppState>,
    req: GetFeedRequest,
) -> Result<Vec<Article>, String> {
    let scrapers = state.scrapers_by_lang.get(&req.lang).ok_or("不支持的语言")?;
    let active_scrapers: Vec<_> = scrapers
        .iter()
        .filter(|s| req.selected_source_ids.contains(&s.id().to_string()))
        .collect();

    if active_scrapers.is_empty() {
        return Err("未选择任何站点".into());
    }

    let excluded = state.emitted_urls.lock().map_err(|e| e.to_string())?.clone();

    let num_scrapers = active_scrapers.len();
    let base_quota = req.limit / num_scrapers;
    let remainder = req.limit % num_scrapers;

    let mut results = Vec::with_capacity(req.limit);

    for (index, scraper) in active_scrapers.iter().enumerate() {
        let quota = if index < remainder {
            base_quota + 1
        } else {
            base_quota
        };

        if quota == 0 {
            continue;
        }

        match scraper.fetch_articles(&state.http_client, quota, &excluded).await {
            Ok(items) => results.extend(items),
            Err(_) => continue,
        }
    }

    {
        let mut emitted = state.emitted_urls.lock().map_err(|e| e.to_string())?;
        for a in &results {
            emitted.insert(a.url.clone());
        }
    }

    let mut final_articles = Vec::with_capacity(results.len());
    for mut a in results {
        if req.mark_familiarity && a.language == "ru" {
            a.words = Some(memory::analyze_text(app.clone(), &a.content));
        } else {
            a.words = None;
        }
        final_articles.push(a);
    }

    Ok(final_articles)
}
