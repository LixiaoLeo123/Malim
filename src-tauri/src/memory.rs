// support Russian only

use rand::seq::SliceRandom;
use rsmorphy::opencorpora::Dictionary;
use rsmorphy::MorphAnalyzer;
use rsmorphy::Source;
use rusqlite::{params, Connection};
use std::collections::HashMap;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use unicode_normalization::UnicodeNormalization;
use std::panic::{catch_unwind, AssertUnwindSafe};

const DEFAULT_S0: f64 = 0.05;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct WordStat {
//     pub lemma: String,
//     pub s0: f64,
//     pub k: u32,       // total times
//     pub last_ts: i64, // timestamp
//     pub current_s: f64,
// }

#[derive(Debug, Clone)]
struct Interaction {
    ts: i64,
    clicked: bool, // true = forget
}

// #[derive(Debug, Serialize)]
// struct GlobalStats {
//     total_expected_vocabulary: f64,
//     total_words: u32,
// }

fn get_db_path(app: &AppHandle) -> PathBuf {
    app.path().app_data_dir().unwrap().join("memory.db")
}

pub fn init_db(app: &AppHandle) -> Result<Connection, String> {
    let path = get_db_path(app);
    let conn = Connection::open(&path).map_err(|e| format!("DB Error: {}", e))?;

    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;",
    )
    .ok();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS interactions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            lemma TEXT NOT NULL,
            ts INTEGER NOT NULL,
            clicked INTEGER NOT NULL,
            synced INTEGER DEFAULT 0,
            UNIQUE(lemma, ts, clicked) 
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS word_stats (
            lemma TEXT PRIMARY KEY,
            s0 REAL NOT NULL,
            k INTEGER NOT NULL,
            last_ts INTEGER NOT NULL,
            current_s REAL NOT NULL,
            dirty INTEGER DEFAULT 0
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS config (
            key TEXT PRIMARY KEY,
            value REAL NOT NULL
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS daily_reading (
            date TEXT PRIMARY KEY,
            count INTEGER NOT NULL
        )",
        [],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_lemma ON interactions(lemma)",
        [],
    )
    .ok();

    Ok(conn)
}

// return: (total likelihood, suprises)
fn calc_likelihood_and_surprise(
    s0: f64,
    alpha: f64,
    intervals: &[(f64, bool)], // (dt_days, clicked)
    weights: &[f64],
) -> (f64, Vec<f64>) {
    let mut total_ll = 0.0;
    let mut surprises = Vec::with_capacity(intervals.len());

    for (i, (dt, clicked)) in intervals.iter().enumerate() {
        let w = weights[i];

        let s = s0 * ((i + 2) as f64).powf(alpha);
        let p = (-dt / s).exp().clamp(1e-9, 1.0 - 1e-9);

        let point_ll = if *clicked { (1.0 - p).ln() } else { p.ln() };

        total_ll += w * point_ll;

        surprises.push(-point_ll);
    }
    (total_ll, surprises)
}

fn fit_s0_weighted(intervals: &[(f64, bool)], alpha: f64, weights: &[f64]) -> f64 {
    let (mut a, mut b) = (0.5, 365.0);
    let phi = (5.0_f64.sqrt() - 1.0) / 2.0;

    let mut c = b - phi * (b - a);
    let mut d = a + phi * (b - a);

    let mut ll_c = calc_weighted_ll_only(c, alpha, intervals, weights);
    let mut ll_d = calc_weighted_ll_only(d, alpha, intervals, weights);

    for _ in 0..40 {
        if ll_c > ll_d {
            b = d;
            d = c;
            ll_d = ll_c;

            c = b - phi * (b - a);
            ll_c = calc_weighted_ll_only(c, alpha, intervals, weights);
        } else {
            a = c;
            c = d;
            ll_c = ll_d;

            d = a + phi * (b - a);
            ll_d = calc_weighted_ll_only(d, alpha, intervals, weights);
        }
    }
    (a + b) / 2.0
}

fn calc_weighted_ll_only(s0: f64, alpha: f64, intervals: &[(f64, bool)], weights: &[f64]) -> f64 {
    let mut ll = 0.0;
    for (i, (dt, clicked)) in intervals.iter().enumerate() {
        let w = weights[i];
        let s = s0 * ((i + 2) as f64).powf(alpha);
        let p = (-dt / s).exp().clamp(1e-9, 1.0 - 1e-9);

        if *clicked {
            ll += w * (1.0 - p).ln();
        } else {
            ll += w * p.ln();
        }
    }
    ll
}

fn calculate_median(mut numbers: Vec<f64>) -> f64 {
    if numbers.is_empty() {
        return DEFAULT_S0;
    }
    numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = numbers.len() / 2;
    if numbers.len() % 2 == 0 {
        (numbers[mid - 1] + numbers[mid]) / 2.0
    } else {
        numbers[mid]
    }
}

// only recompute dirty s0 without alpha
fn recompute_all(conn: &mut Connection) -> Result<(), String> {
    let current_alpha: f64 = conn
        .query_row("SELECT value FROM config WHERE key='alpha'", [], |row| {
            row.get(0)
        })
        .unwrap_or(0.3);

    let fallback_s0: f64 = conn
        .query_row(
            "SELECT value FROM config WHERE key='fallback_s0'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(DEFAULT_S0);

    let mut raw_data: HashMap<String, Vec<Interaction>> = HashMap::new();

    {
        let mut stmt = conn
            .prepare(
                "SELECT i.lemma, i.ts, i.clicked 
             FROM interactions i
             LEFT JOIN word_stats w ON i.lemma = w.lemma
             WHERE w.dirty = 1 OR w.lemma IS NULL
             ORDER BY i.ts ASC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for r in rows {
            let (lemma, ts, clicked) = r.map_err(|e| e.to_string())?;
            raw_data.entry(lemma).or_default().push(Interaction {
                ts,
                clicked: clicked == 1,
            });
        }
    }

    if raw_data.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let mut dataset: Vec<(String, Vec<(f64, bool)>, i64)> = Vec::new();
    for (lemma, records) in raw_data {
        if records.is_empty() {
            continue;
        }

        if records.len() == 1 {
            let last_ts = records[0].ts;
            let s0 = fallback_s0;
            let k = 1;
            let current_s = s0 * (k as f64).powf(current_alpha);

            tx.execute(
                "INSERT OR REPLACE INTO word_stats (lemma, s0, k, last_ts, current_s, dirty) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
                params![lemma, s0, k, last_ts, current_s]
            ).map_err(|e| e.to_string())?;
            continue;
        }
        let last_ts = records.last().unwrap().ts;
        let mut intervals = Vec::new();

        for i in 1..records.len() {
            let dt = (records[i].ts - records[i - 1].ts) as f64 / 86400.0;
            if dt > 0.0 {
                intervals.push((dt, records[i].clicked));
            }
        }

        if !intervals.is_empty() {
            dataset.push((lemma, intervals, last_ts));
        }
    }

    for (lemma, intervals, last_ts) in &dataset {
        let mut weights = vec![1.0; intervals.len()];

        let has_forget = intervals.iter().any(|&(_, clicked)| clicked);
        let has_remember = intervals.iter().any(|&(_, clicked)| !clicked);
        let is_valid_for_mle = intervals.len() >= 5 && has_forget && has_remember;

        let mut s0 = fallback_s0;

        if is_valid_for_mle {
            for _ in 0..3 {
                s0 = fit_s0_weighted(intervals, current_alpha, &weights);
                let (_, surprises) =
                    calc_likelihood_and_surprise(s0, current_alpha, intervals, &weights);
                for (w, surp) in weights.iter_mut().zip(surprises.iter()) {
                    if *surp > 3.0 {
                        *w *= 0.3;
                    }
                }
            }
        }

        let mut k = 1;
        for (i, (_, _)) in intervals.iter().enumerate() {
            if weights[i] > 0.5 {
                k += 1;
            }
        }

        let current_s = s0 * (k as f64).powf(current_alpha);

        tx.execute(
            "INSERT OR REPLACE INTO word_stats (lemma, s0, k, last_ts, current_s, dirty) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![lemma, s0, k, last_ts, current_s]
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn calibrate_global_model(conn: &mut Connection) -> Result<(), String> {
    let mut raw_data: HashMap<String, Vec<Interaction>> = HashMap::new();
    {
        let mut stmt = conn
            .prepare("SELECT lemma, ts, clicked FROM interactions ORDER BY ts ASC")
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i32>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        for r in rows {
            let (lemma, ts, clicked) = r.map_err(|e| e.to_string())?;
            raw_data.entry(lemma).or_default().push(Interaction {
                ts,
                clicked: clicked == 1,
            });
        }
    }

    if raw_data.is_empty() {
        return Ok(());
    }

    let mut dataset: Vec<(String, Vec<(f64, bool)>, i64)> = Vec::with_capacity(raw_data.len());
    for (lemma, records) in raw_data {
        if records.is_empty() {
            continue;
        }

        let last_ts = records.last().unwrap().ts;
        let mut intervals = Vec::with_capacity(records.len() - 1);

        for i in 1..records.len() {
            let dt = (records[i].ts - records[i - 1].ts) as f64 / 86400.0;
            if dt > 0.0 {
                intervals.push((dt, records[i].clicked));
            }
        }

        dataset.push((lemma, intervals, last_ts));
    }

    let valid_dataset: Vec<_> = dataset
        .iter()
        .filter(|(_, intervals, _)| {
            let has_forget = intervals.iter().any(|&(_, c)| c);
            let has_remember = intervals.iter().any(|&(_, c)| !c);
            intervals.len() >= 5 && has_forget && has_remember
        })
        .collect();

    let alpha_candidates: Vec<f64> = (10..=60).step_by(2).map(|x| x as f64 / 100.0).collect();
    let mut best_alpha = 0.3;

    if !valid_dataset.is_empty() {
        let mut best_total_ll = f64::MIN;
        for &test_alpha in &alpha_candidates {
            let mut total_ll = 0.0;

            for (_, intervals, _) in &valid_dataset {
                let mut weights = vec![1.0; intervals.len()];
                let mut s0 = DEFAULT_S0;

                for _ in 0..3 {
                    s0 = fit_s0_weighted(intervals, test_alpha, &weights);
                    let (_, surprises) =
                        calc_likelihood_and_surprise(s0, test_alpha, intervals, &weights);
                    for (w, surp) in weights.iter_mut().zip(surprises.iter()) {
                        if *surp > 3.0 {
                            *w *= 0.3;
                        }
                    }
                }
                total_ll += calc_weighted_ll_only(s0, test_alpha, intervals, &weights);
            }

            if total_ll > best_total_ll {
                best_total_ll = total_ll;
                best_alpha = test_alpha;
            }
        }
    }

    let mut valid_s0s = Vec::new();
    let mut fitted_data = HashMap::new();

    for (lemma, intervals, _) in &dataset {
        let has_forget = intervals.iter().any(|&(_, c)| c);
        let has_remember = intervals.iter().any(|&(_, c)| !c);
        let is_valid_for_mle = intervals.len() >= 5 && has_forget && has_remember;

        if is_valid_for_mle {
            let mut weights = vec![1.0; intervals.len()];
            let mut s0 = DEFAULT_S0;
            for _ in 0..3 {
                s0 = fit_s0_weighted(intervals, best_alpha, &weights);
                let (_, surprises) =
                    calc_likelihood_and_surprise(s0, best_alpha, intervals, &weights);
                for (w, surp) in weights.iter_mut().zip(surprises.iter()) {
                    if *surp > 3.0 {
                        *w *= 0.3;
                    }
                }
            }
            valid_s0s.push(s0);
            fitted_data.insert(lemma.clone(), (s0, weights));
        }
    }

    let new_fallback_s0 = calculate_median(valid_s0s);

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('alpha', ?1)",
        params![best_alpha],
    )
    .map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES ('fallback_s0', ?1)",
        params![new_fallback_s0],
    )
    .map_err(|e| e.to_string())?;

    for (lemma, intervals, last_ts) in dataset {
        let (s0, weights) = if let Some((fitted_s0, w)) = fitted_data.remove(&lemma) {
            (fitted_s0, w)
        } else {
            (new_fallback_s0, vec![1.0; intervals.len()])
        };

        let mut k = 1;
        for (i, _) in intervals.iter().enumerate() {
            if weights[i] > 0.5 {
                k += 1;
            }
        }

        let current_s = s0 * (k as f64).powf(best_alpha);

        tx.execute(
            "INSERT OR REPLACE INTO word_stats (lemma, s0, k, last_ts, current_s, dirty) VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            params![lemma, s0, k, last_ts, current_s]
        ).map_err(|e| e.to_string())?;
    }

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn record_word_click(app: AppHandle, lemma: String, clicked: bool) -> Result<(), String> {
    let lemma: String = lemma
        .nfd()
        .filter(|c| {
            let cp = *c as u32;
            !(0x0300..=0x036F).contains(&cp)
        })
        .collect();

    let lemma = lemma.to_lowercase();

    let contains_non_cyrillic = lemma.chars().any(|c| {
        let cp = c as u32;
        !(0x0400..=0x04FF).contains(&cp)
    });

    if contains_non_cyrillic {
        return Ok(());
    }

    if lemma.is_empty() {
        return Ok(());
    }

    let mut conn = init_db(&app)?;
    let now = chrono::Local::now().timestamp();

    let tx = conn.transaction().map_err(|e| e.to_string())?;

    tx.execute(
        "INSERT INTO interactions (lemma, ts, clicked) VALUES (?1, ?2, ?3)",
        params![&lemma, now, clicked],
    )
    .map_err(|e| e.to_string())?;

    // *0.5 is a temporary solution
    tx.execute(
        "UPDATE word_stats SET last_ts = ?1, current_s = current_s * 0.5, dirty = 1 WHERE lemma = ?2", 
        params![now, &lemma]
    ).map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn record_unparsed_text_words(app: AppHandle, text: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let mut conn = init_db(&app)?;
        let morph = {
            let dict_path = crate::memory::ensure_dict_files(&app);
            let dict = Dictionary::from_file(dict_path);
            MorphAnalyzer::new(dict)
        };
        let now = chrono::Local::now().timestamp();
        
        let tx = conn.transaction().map_err(|e| e.to_string())?;

        for original_token in text.split_whitespace() {
            let clean_word: String = original_token
                .chars()
                .filter(|c| c.is_alphabetic() || *c == '-')
                .collect::<String>()
                .to_lowercase();
                
            if clean_word.is_empty() { continue; }
            
            let parse_result = catch_unwind(AssertUnwindSafe(|| {
                morph.parse(&clean_word)
            }));

            let lemma = match parse_result {
                Ok(parsed_vec) => {
                    match parsed_vec.first() {
                        Some(p) => p.lex.get_lemma(&morph).to_string(),
                        None => clean_word.clone(),
                    }
                }
                Err(_) => {
                    clean_word.clone()
                }
            };

            let lemma = lemma.to_lowercase();
            let contains_non_cyrillic = lemma.chars().any(|c| {
                let cp = c as u32;
                !(0x0400..=0x04FF).contains(&cp)
            });
            if contains_non_cyrillic { continue; }
            if lemma.is_empty() { continue; }

            // Ignore uniqueness constraint errors (we might insert same lemma twice or already clicked in the exact same second)
            tx.execute(
                "INSERT OR IGNORE INTO interactions (lemma, ts, clicked) VALUES (?1, ?2, 0)",
                params![&lemma, now],
            ).ok();

            tx.execute(
                "UPDATE word_stats SET last_ts = ?1, current_s = current_s * 0.5, dirty = 1 WHERE lemma = ?2", 
                params![now, &lemma]
            ).ok();
        }
        
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn run_global_calibration(app: AppHandle) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let mut conn = init_db(&app)?;
        let start = std::time::Instant::now();

        calibrate_global_model(&mut conn)?;

        let duration = start.elapsed();
        Ok(format!("Global Calibration Complete. Took: {:?}", duration))
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
pub fn get_alpha(app: AppHandle) -> Result<f64, String> {
    let conn = init_db(&app).map_err(|e| e.to_string())?;
    match conn.query_row("SELECT value FROM config WHERE key = 'alpha'", [], |row| {
        row.get::<_, f64>(0)
    }) {
        Ok(val) => Ok(val),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0.3),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_vocabulary_expectation(app: AppHandle) -> Result<f64, String> {
    let mut conn = init_db(&app)?;
    let now = chrono::Local::now().timestamp();

    recompute_all(&mut conn)?;

    let mut stmt = conn
        .prepare("SELECT current_s, last_ts FROM word_stats")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?)))
        .map_err(|e| e.to_string())?;

    let mut total_p = 0.0;
    for r in rows {
        let (s, last_ts) = r.map_err(|e| e.to_string())?;
        if s > 0.0 {
            let dt = (now - last_ts) as f64 / 86400.0;
            total_p += (-dt / s).exp();
        }
    }
    Ok(total_p)
}

#[tauri::command]
pub async fn update_daily_reading(app: AppHandle, count: u32) -> Result<(), String> {
    let conn = init_db(&app)?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    conn.execute(
        "INSERT INTO daily_reading (date, count) VALUES (?1, ?2) ON CONFLICT(date) DO UPDATE SET count = count + ?2",
        params![today, count]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_daily_reading(app: AppHandle) -> Result<u32, String> {
    let conn = init_db(&app).map_err(|e| e.to_string())?;
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let result: Result<u32, rusqlite::Error> = conn.query_row(
        "SELECT count FROM daily_reading WHERE date = ?1",
        params![today],
        |row| row.get(0),
    );
    match result {
        Ok(count) => Ok(count),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn get_reading_by_date(app: AppHandle, date_str: String) -> Result<u32, String> {
    let conn = init_db(&app).map_err(|e| e.to_string())?;
    let result: Result<u32, rusqlite::Error> = conn.query_row(
        "SELECT count FROM daily_reading WHERE date = ?1",
        params![date_str],
        |row| row.get(0),
    );
    match result {
        Ok(count) => Ok(count),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub async fn get_words_in_p_range(
    app: AppHandle,
    p_min: f64,
    p_max: f64,
    limit: usize,
) -> Result<Vec<String>, String> {
    let conn = init_db(&app)?;
    let now = chrono::Local::now().timestamp();

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

    let mut matched_words = Vec::new();

    for r in rows {
        let (lemma, s, last_ts) = r.map_err(|e| e.to_string())?;
        let dt = (now - last_ts) as f64 / 86400.0;
        let p = (-dt / s).exp();

        if p >= p_min && p <= p_max {
            matched_words.push(lemma);
        }
    }

    let mut rng = rand::thread_rng();
    matched_words.shuffle(&mut rng);
    matched_words.truncate(limit);

    Ok(matched_words)
}



pub fn ensure_dict_files(app: &tauri::AppHandle) -> String {
    use tauri::Manager;
    let app_data = app.path().app_data_dir().expect("Failed to get app_data_dir");
    let out_dir = app_data.join("rsmorphy_data");
    
    if !out_dir.join("meta.json.gz").exists() {
        std::fs::create_dir_all(&out_dir).unwrap_or_else(|e| {
            eprintln!("Failed to create rsmorphy_data dir: {}", e);
        });
        
        macro_rules! dump {
            ($file:expr) => {
                if let Err(e) = std::fs::write(out_dir.join($file), include_bytes!(concat!("../rsmorphy_data/", $file))) {
                    eprintln!("Failed to write rsmorphy dict {}: {}", $file, e);
                }
            };
        }
        
        dump!("grammemes.json.gz");
        dump!("gramtab-opencorpora-ext.json.gz");
        dump!("gramtab-opencorpora-int.json.gz");
        dump!("meta.json.gz");
        dump!("paradigms.array.gz");
        dump!("prediction-prefixes.dawg.gz");
        dump!("prediction-suffixes-0.dawg.gz");
        dump!("prediction-suffixes-1.dawg.gz");
        dump!("prediction-suffixes-2.dawg.gz");
        dump!("p_t_given_w.intdawg.gz");
        dump!("suffixes.json.gz");
        dump!("words.dawg.gz");
    }
    
    out_dir.to_string_lossy().to_string()
}

pub fn analyze_text(app: AppHandle, text: &str) -> Vec<(String, Option<f64>)> {
    let morph = Some({
        let dict_path = ensure_dict_files(&app);
        let dict = Dictionary::from_file(&dict_path);
        MorphAnalyzer::new(dict)
    });
    let conn = init_db(&app).ok();

    let now = chrono::Local::now().timestamp();
    let mut results = Vec::new();

    for original_token in text.split_whitespace() {
        let clean_word: String = original_token
            .chars()
            .filter(|c| c.is_alphabetic() || *c == '-')
            .collect::<String>()
            .to_lowercase();
        
        let p = if clean_word.is_empty() {
            None
        } else if let (Some(m), Some(c)) = (&morph, &conn) {
            let parse_result = catch_unwind(AssertUnwindSafe(|| {
                m.parse(&clean_word)
            }));

            let lemma = match parse_result {
                Ok(parsed_vec) => {
                    match parsed_vec.first() {
                        Some(p) => p.lex.get_lemma(m).to_string(),
                        None => clean_word.clone(),
                    }
                }
                Err(_) => {
                    clean_word.clone()
                }
            };
            // dbg!(&lemma);
            // dbg!(&original_token);
            let (current_s, last_ts): (f64, i64) = c
                .query_row(
                    "SELECT current_s, last_ts FROM word_stats WHERE lemma = ?1",
                    [&lemma],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap_or((0.0, 0));

            if current_s > 0.0 {
                let dt = (now - last_ts) as f64 / 86400.0;
                Some((-dt / current_s).exp().min(1.0))
            } else {
                None
            }
        } else {
            None
        };

        results.push((original_token.to_string(), p));
    }

    results
}