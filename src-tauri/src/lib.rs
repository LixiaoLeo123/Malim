use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use msedge_tts::tts::{client::connect, SpeechConfig};
use msedge_tts::voice::Voice as EdgeVoice;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordBlock {
    text: String,
    pos: String,
    definition: String,
    chinese_root: Option<String>,
    grammar_note: Option<String>,
    audio_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sentence {
    id: String,
    original: String,
    blocks: Vec<WordBlock>,
    translation: String,
    audio_path: Option<String>,
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    id: String,
    current: usize,
    total: usize,
    percent: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiParsedResult {
    translation: String,
    blocks: Vec<WordBlock>,
}

struct AppState {
    // sentence_cache: Mutex<HashMap<String, Vec<WordBlock>>>,
}

// --- TTS ---
fn pick_voice(lang: &str) -> &'static str {
    match lang {
        "KR" => "ko-KR-SunHiNeural",
        "RU" => "ru-RU-SvetlanaNeural",
        _ => "en-US-JennyNeural",
    }
}

fn hash_key(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn audio_dir(app: &AppHandle, article_id: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir error: {}", e))?
        .join("audio")
        .join(article_id);
    fs::create_dir_all(&dir).map_err(|e| format!("create audio dir error: {}", e))?;
    Ok(dir)
}

fn edge_tts_mp3(text: &str, voice_name: &str) -> Result<Vec<u8>, String> {
    let mut client = connect().map_err(|e| format!("edge tts connect error: {}", e))?;

    let voice_json = format!(r#"{{"Name":"{}"}}"#, voice_name);
    let voice: EdgeVoice =
        serde_json::from_str(&voice_json).map_err(|e| format!("voice parse error: {}", e))?;

    let config = SpeechConfig::from(&voice);

    let audio = client
        .synthesize(text, &config)
        .map_err(|e| format!("edge tts synthesize error: {}", e))?;

    Ok(audio.audio_bytes)
}

fn ensure_audio_cached(
    app: &AppHandle,
    article_id: &str,
    lang: &str,
    text: &str,
    kind: &str, // "sentence" | "block"
) -> Result<String, String> {
    let voice = pick_voice(lang);
    let key = hash_key(&format!("{}|{}|{}", voice, kind, text));
    let dir = audio_dir(app, article_id)?;
    let path = dir.join(format!("{}_{}.mp3", kind, key));

    if Path::new(&path).exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let audio = edge_tts_mp3(text, voice)?;
    fs::write(&path, audio).map_err(|e| format!("write audio error: {}", e))?;
    Ok(path.to_string_lossy().to_string())
}

// --- AI ---
fn build_prompt(lang: &str, sentence: &str) -> String {
    let lang_target = if lang == "KR" { "Korean" } else { "Russian" };

    let pos_rules = if lang == "KR" {
        r#"
POS Tagging (Strictly use these tags only):
- noun: Common nouns, proper nouns.
- pronoun: I, you, he, this, that.
- verb: Action verbs.
- adjective: Descriptive verbs.
- adverb: Modifying verbs/adjectives.
- particle: Case markers, topic markers.
- ending: Verb endings, connectives.
- punctuation: Periods (.), commas (,), question marks (?), exclamation marks (!).
- unknown: If unable to determine.
"#
    } else {
        "Standard Russian POS tagging (noun, verb, adj, adv, prep, conj, punctuation, etc.)."
    };

    let specific_rule = if lang == "KR" {
        "For Sino-Korean words, providing 'chinese_root' is MANDATORY."
    } else {
        "For 'chinese_root', always return null."
    };

    format!(
        r#"
You are a precise JSON generator.
STRICT RULES:
1. Output MUST be a valid JSON object containing "translation" and "blocks" keys.
2. Do NOT use Markdown code blocks.
3. Use ONLY standard ASCII punctuation.
4. Do NOT include any text outside the JSON array.

Task: Analyze the {lang_target} sentence below.
1. TRANSLATION: Provide a natural English translation of the entire sentence.
2. TOKENIZATION: Split into morphemes.
   - CRITICAL: Do NOT decompose Hangul characters (Jamo).
   - CRITICAL: Do NOT discard punctuation. Output punctuation as separate blocks with pos 'punctuation'.
2. {pos_rules}
3. Provide concise English definition.
4. {specific_rule}

Example Output:
{{
    "translation": "I go to school.",
    "blocks": [
        {{ "text": "학교", "pos": "noun", "definition": "school", "chinese_root": "学校", "grammar_note": null }},
        {{ "text": ".", "pos": "punctuation", "definition": ".", "chinese_root": null, "grammar_note": null }}
    ]
}}

Sentence: "{sentence}"
Output:
"#
    )
}

async fn call_ai_api(
    api_key: &str,
    api_url: &str,
    model_name: &str,
    prompt: String,
) -> Result<AiParsedResult, String> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": model_name,
        "messages": [
            {"role": "system", "content": "You are a helpful assistant that outputs only JSON."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.1,
        "stream": false,
        "max_tokens": 8196,
        "enable_thinking": false
    });

    let res = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| format!("Network Error: {}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res
            .text()
            .await
            .unwrap_or_else(|_| "Cannot read response body".to_string());
        return Err(format!("API Error Code: {}, Body: {}", status, text));
    }

    let response_text = res
        .text()
        .await
        .map_err(|e| format!("Read Body Error: {}", e))?;

    dbg!("----- API Raw Response -----");
    dbg!(&response_text);
    dbg!("---------------------------");

    let json_res: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("JSON Parse Error: {}. Raw text: {}", e, response_text))?;

    let content = json_res["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("API returned an empty or invalid content field.")?;
    let clean_content = content
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();

    let ai_parsed_result: AiParsedResult = serde_json::from_str(clean_content)
        .map_err(|e| format!("Invalid JSON Structure: {}", e))?;
    Ok(ai_parsed_result)
}

//major func
#[tauri::command]
async fn parse_text(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    text: String,
    language: String,
    api_key: String,
    api_url: String,
    model_name: String,
    concurrency: usize,
    old_sentences: Option<Vec<Sentence>>, //as cache in edit mode
) -> Result<Vec<Sentence>, String> {
    if api_key.is_empty() {
        return Err("API Key is missing".to_string());
    }

    let mut old_map: HashMap<String, Sentence> = HashMap::new();
    if let Some(old) = old_sentences {
        for sent in old {
            old_map.insert(sent.original.clone(), sent);
        }
    }
    let old_map = Arc::new(old_map);
    let raw_sentences: Vec<String> = text
        .split_inclusive(|c| matches!(c, '.' | '。' | '!' | '?' | '\n'))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    dbg!(&raw_sentences);

    let total = raw_sentences.len();

    let api_key = Arc::new(api_key);
    let api_url = Arc::new(api_url);
    let model_name = Arc::new(model_name);
    let language = Arc::new(language);
    let id = Arc::new(id);
    let completed = Arc::new(AtomicUsize::new(0));

    let tasks = raw_sentences.into_iter().enumerate().map(|(i, raw)| {
        let api_key = Arc::clone(&api_key);
        let api_url = Arc::clone(&api_url);
        let model_name = Arc::clone(&model_name);
        let language = Arc::clone(&language);
        let id = Arc::clone(&id);
        let old_map = Arc::clone(&old_map);
        let completed = Arc::clone(&completed);
        let app = app.clone();

        async move {
            let cached = old_map.get(&raw).cloned();
            let (blocks, translation) = if let Some(b) = cached {
                (b.blocks, b.translation)
            } else {
                let prompt = build_prompt(&language, &raw);
                match call_ai_api(&api_key, &api_url, &model_name, prompt).await {
                    Ok(res) => (res.blocks, res.translation),
                    Err(err) => (
                        vec![WordBlock {
                            text: raw.clone(),
                            pos: "error".to_string(),
                            definition: format!("Error: {}", err),
                            chinese_root: None,
                            grammar_note: None,
                            audio_path: None
                        }],
                        "Translation unavailable due to error.".to_string(),
                    ),
                }
            };
            // --- audio ---
            for b in blocks.iter_mut() {
                if b.pos == "punctuation" || b.text.trim().is_empty() {
                    b.audio_path = None;
                    continue;
                }
                match ensure_audio_cached(&app, &id, &language, &b.text, "block") {
                    Ok(p) => b.audio_path = Some(p),
                    Err(err) => {
                        b.audio_path = None;
                        dbg!(&err);
                    }
                }
            }

            let sentence_audio = ensure_audio_cached(&app, &id, &language, &raw, "sentence")
                .ok();

            let sentence = Sentence {
                id: format!("{}_{}", id, i),
                original: raw,
                blocks,
                translation,
                audio_path: sentence_audio,
            };

            let current = completed.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = app.emit(
                "parsing-progress",
                ProgressPayload {
                    id: id.to_string(),
                    current,
                    total,
                    percent: ((current as f32 / total as f32) * 100.0) as u32,
                },
            );

            (i, sentence)
        }
    });

    let mut unordered_results: Vec<(usize, Sentence)> = stream::iter(tasks)
        .buffer_unordered(concurrency)
        .collect()
        .await;

    unordered_results.sort_by_key(|(i, _)| *i);
    let results: Vec<Sentence> = unordered_results.into_iter().map(|(_, s)| s).collect();

    Ok(results)
}

#[derive(Serialize, Deserialize)]
struct AppData {
    articles: serde_json::Value,
    draft: serde_json::Value,
    api_key: String,
}

#[tauri::command]
fn save_data(app: AppHandle, data: String) {
    let app_data_dir: PathBuf = app
        .path()
        .app_data_dir()
        .expect("failed to get app_data_dir");

    let path = app_data_dir.join("data.json");

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    fs::write(&path, data).expect("failed to write data.json");
}

#[tauri::command]
fn load_data(app: AppHandle) -> String {
    let app_data_dir: PathBuf = app
        .path()
        .app_data_dir()
        .expect("failed to get app_data_dir");

    let path = app_data_dir.join("data.json");

    if path.exists() {
        fs::read_to_string(&path).expect("failed to read data.json")
    } else {
        "{}".to_string()
    }
}

#[tauri::command]
fn delete_article_audio(app: AppHandle, article_id: String) -> Result<(), String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir error: {}", e))?
        .join("audio")
        .join(article_id);

    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| format!("remove audio dir error: {}", e))?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            // sentence_cache: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![parse_text, save_data, load_data, delete_article_audio])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
