use super::{Article, SourceInfo};
use crate::memory;
use crate::state::AppState;
use futures::future::join_all;
use rand::prelude::SliceRandom;
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
pub fn clear_emitted_urls(state: State<'_, AppState>) -> Result<(), String> {
    state
        .emitted_urls
        .lock()
        .map_err(|e| e.to_string())?
        .clear();
    Ok(())
}

#[tauri::command]
pub async fn get_feed(
    app: AppHandle,
    state: State<'_, AppState>,
    req: GetFeedRequest,
) -> Result<Vec<Article>, String> {
    let scrapers = state
        .scrapers_by_lang
        .get(&req.lang)
        .ok_or("Unsupported language")?;
    let active_scrapers: Vec<_> = scrapers
        .iter()
        .filter(|s| req.selected_source_ids.contains(&s.id().to_string()))
        .collect();

    if active_scrapers.is_empty() {
        return Err("No sources selected".into());
    }

    let excluded = state
        .emitted_urls
        .lock()
        .map_err(|e| e.to_string())?
        .clone();

    let mut indices: Vec<usize> = (0..active_scrapers.len()).collect();

    indices.shuffle(&mut rand::thread_rng());

    let n_scrapers = indices.len();
    if n_scrapers == 0 {
        return Ok(Vec::new());
    }

    let base_quota = req.limit / n_scrapers;
    let remainder = req.limit % n_scrapers;

    let tasks: Vec<_> = indices
        .iter()
        .enumerate()
        .filter_map(|(rank, &idx)| {
            let quota = if rank < remainder {
                base_quota + 1
            } else {
                base_quota
            };

            if quota == 0 {
                return None;
            }

            let scraper = &active_scrapers[idx];
            Some((scraper, quota))
        })
        .collect();

    let futures = tasks.into_iter().map(|(scraper, quota)| {
        scraper.fetch_articles(&state.http_client, quota, &excluded)
    });

    let results = join_all(futures)
        .await
        .into_iter()
        .filter_map(|res| res.ok())
        .flatten()
        .collect::<Vec<_>>();

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
            let valid_scores: Vec<f64> = a
                .words
                .clone()
                .unwrap()
                .iter()
                .filter_map(|(_word, score)| *score)
                .collect();

            if !valid_scores.is_empty() {
                let sum: f64 = valid_scores.iter().sum();
                let average = sum / valid_scores.len() as f64;
                a.difficulty = Some(1.0 - average);
            } else {
                a.difficulty = None;
            }

            let total_valid_count = valid_scores.len();

            if total_valid_count > 0 {
                let match_count = valid_scores
                    .iter()
                    .filter(|&&score| (0.4..=0.7).contains(&score))
                    .count();

                a.recommendation = Some(match_count as f64 / total_valid_count as f64);
            } else {
                a.recommendation = Some(0.0);
            }
        } else {
            a.words = None;
        }
        final_articles.push(a);
    }
    // dbg!(&final_articles);
    Ok(final_articles)
}
