use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::memory::init_db;
use tauri::{AppHandle};

#[derive(Serialize)]
pub struct BrainWord {
    pub lemma: String,
    pub s: f64,
    pub p: f64,
}

#[tauri::command]
pub fn get_brain_words(app: AppHandle) -> Result<Vec<BrainWord>, String> {
    dbg!("Fetching brain words...");
    let conn = init_db(&app)?;
    dbg!("Database connection established.");
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    let mut stmt = conn
        .prepare("SELECT lemma, current_s, last_ts FROM word_stats WHERE current_s > 0.0")
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, f64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut words = Vec::new();

    for r in rows {
        let (lemma, s, last_ts) = r.map_err(|e| e.to_string())?;
        let dt = (now - last_ts) as f64 / 86400.0; 
        let p = (-dt / s).exp(); 
        
        words.push(BrainWord { lemma, s, p });
    }

    Ok(words)
}