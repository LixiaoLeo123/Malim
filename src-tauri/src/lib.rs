use dashmap::DashMap;
use futures::stream::{self, StreamExt};
use msedge_tts::tts::{client::connect, SpeechConfig};
use msedge_tts::voice::Voice as EdgeVoice;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::{
    sync::{Mutex, Semaphore},
    task,
};

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

fn ensure_audio_cached_sync(
    app: &AppHandle,
    article_id: &str,
    lang: &str,
    text: &str,
    kind: &str,
) -> Result<String, String> {
    let voice = pick_voice(lang);
    let key = hash_key(&format!("{}|{}|{}", voice, kind, text));
    let dir = audio_dir(app, article_id)?;
    let path = dir.join(format!("{}_{}.mp3", kind, key));

    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let audio = edge_tts_mp3(text, voice)?;

    let tmp = dir.join(format!(".tmp_{}_{}.mp3", kind, key));
    fs::write(&tmp, audio).map_err(|e| format!("write audio error: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename audio error: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

async fn ensure_audio_cached(
    app: AppHandle,
    article_id: Arc<String>,
    lang: Arc<String>,
    text: String,
    kind: &'static str,
    tts_sem: Arc<Semaphore>,
    tts_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
) -> Result<String, String> {
    let voice = pick_voice(&lang);
    let key = hash_key(&format!("{}|{}|{}", voice, kind, text));

    let lock = tts_locks
        .entry(key.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();

    let _guard = lock.lock().await;

    let dir = audio_dir(&app, &article_id)?;
    let path = dir.join(format!("{}_{}.mp3", kind, key));

    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
    }

    let _permit = tts_sem
        .acquire_owned()
        .await
        .map_err(|_| "tts semaphore closed".to_string())?;

    let app2 = app.clone();
    let article_id2 = article_id.clone();
    let lang2 = lang.clone();
    let text2 = text.clone();

    let out_path = task::spawn_blocking(move || {
        ensure_audio_cached_sync(&app2, &article_id2, &lang2, &text2, kind)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))??;

    tts_locks.remove(&key);

    Ok(out_path)
}

// --- AI ---
fn build_prompt(lang: &str, sentence: &str) -> String {
    const BASE_RULES: &str = r#"
You are a precise JSON generator.
STRICT RULES:
1. Your output MUST be a single, valid JSON object.
2. Do NOT use Markdown code blocks (```json ... ```) in your final output.
3. Use ONLY standard ASCII punctuation.
4. Do NOT include any text outside the JSON object.
"#;

    let (lang_target, task_rules, example) = if lang == "KR" {
        let kr_rules = r#"
Task: Analyze the Korean sentence below.
1.  TRANSLATION: Provide a natural English translation of the entire sentence.
2.  TOKENIZATION: Split into morphemes.
    - CRITICAL: Do NOT decompose Hangul characters (Jamo).
    - CRITICAL: Do NOT discard punctuation. Output punctuation as separate blocks with pos 'punctuation'.
3.  POS Tagging (Strictly use these tags only):
    - noun, pronoun, verb, adjective, adverb, particle, ending, punctuation, unknown.
4.  DEFINITION: Provide a concise English definition for each block.
5.  CHINESE ROOT: For Sino-Korean words, providing 'chinese_root' is MANDATORY.
"#;
        let kr_example = r#"
Example Output:
{{
    "translation": "I go to school.",
    "blocks": [
        {{ "text": "학교", "pos": "noun", "definition": "school", "chinese_root": "学校", "grammar_note": null }},
        {{ "text": "에", "pos": "particle", "definition": "to (indicates direction)", "chinese_root": null, "grammar_note": "Location marker" }},
        {{ "text": "갑니다", "pos": "verb", "definition": "go", "chinese_root": null, "grammar_note": "Formal, present tense" }},
        {{ "text": ".", "pos": "punctuation", "definition": ".", "chinese_root": null, "grammar_note": null }}
    ]
}}
"#;
        ("Korean", kr_rules, kr_example)
    } else {
        let ru_rules = r#"
You are an expert Russian linguist who ALWAYS follows JSON formatting rules.

Task: Analyze the Russian sentence below.

CRITICAL, NON-NEGOTIABLE RULES:
1.  **GRAMMAR NOTE (MANDATORY!)**: For EVERY word block, you MUST provide a detailed 'grammar_note'. This is the most important instruction. Do not skip it or leave it null (except for punctuation).
    -   For **Nouns, Pronouns, Adjectives**: MUST specify `Case`, `Number`, `Gender`.
    -   For **Verbs**: MUST specify its `Lemma` (infinitive form), `Aspect` (Perfective/Imperfective), `Tense`, `Person`, and `Number`.
2.  **LEMMATIZATION**: Identify the base/dictionary form (lemma) of each word. The definition should be for the lemma.
3.  **TRANSLATION**: Provide a natural English translation of the entire sentence.
4.  **TOKENIZATION**: Split into words and punctuation. Punctuation marks are separate blocks.
5.  **POS TAGGING**: Use standard tags (e.g., noun, verb, adj, adv, pron, prep, conj, particle, punctuation).
"#;
        let ru_example = r#"
Example Output (Follow this structure EXACTLY):
{{
    "translation": "I am reading an interesting book.",
    "blocks": [
        {{ "text": "Я", "pos": "pron", "definition": "I", "grammar_note": "Case: Nominative, Person: 1st, Number: Singular" }},
        {{ "text": "читаю", "pos": "verb", "definition": "read", "grammar_note": "Lemma: читать, Aspect: Imperfective, Tense: Present, Person: 1st, Number: Singular" }},
        {{ "text": "интересную", "pos": "adj", "definition": "interesting", "grammar_note": "Lemma: интересный, Case: Accusative, Number: Singular, Gender: Feminine" }},
        {{ "text": "книгу", "pos": "noun", "definition": "book", "grammar_note": "Lemma: книга, Case: Accusative, Number: Singular, Gender: Feminine, Animacy: Inanimate" }},
        {{ "text": ".", "pos": "punctuation", "definition": ".", "grammar_note": null }}
    ]
}}
"#;
        ("Russian", ru_rules, ru_example)
    };

    format!(
        r#"{base_rules}
{task_rules}
{example}
Sentence: "{sentence}"
Output:"#,
        base_rules = BASE_RULES,
        task_rules = task_rules,
        example = example,
        sentence = sentence
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
    pre_cache_audio: bool,
    tts_concurrency: usize,
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

    dbg!(&language);
    dbg!(&raw_sentences);

    let total = raw_sentences.len();

    let api_key = Arc::new(api_key);
    let api_url = Arc::new(api_url);
    let model_name = Arc::new(model_name);
    let language = Arc::new(language);
    let id = Arc::new(id);
    let completed = Arc::new(AtomicUsize::new(0));
    let tts_sem = Arc::new(Semaphore::new(tts_concurrency.max(1)));
    let tts_locks: Arc<DashMap<String, Arc<Mutex<()>>>> = Arc::new(DashMap::new());

    let tasks = raw_sentences.into_iter().enumerate().map(|(i, raw)| {
        let api_key = api_key.clone();
        let api_url = api_url.clone();
        let model_name = model_name.clone();
        let language = language.clone();
        let id = id.clone();
        let old_map = old_map.clone();
        let completed = completed.clone();
        let app = app.clone();

        let tts_locks = tts_locks.clone();
        let tts_sem = tts_sem.clone();

        async move {
            let cached = old_map.get(&raw).cloned();
            let (mut blocks, translation) = if let Some(b) = cached {
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
                            audio_path: None,
                        }],
                        "Translation unavailable due to error.".to_string(),
                    ),
                }
            };

            // --- audio ---
            let mut sentence_audio = None;
            if pre_cache_audio {
                let sentence_handle = {
                    let app2 = app.clone();
                    let id2 = id.clone();
                    let lang2 = language.clone();
                    let sem2 = tts_sem.clone();
                    let raw2 = raw.clone();
                    let locks2 = tts_locks.clone();

                    tokio::spawn(async move {
                        ensure_audio_cached(app2, id2, lang2, raw2, "sentence", sem2, locks2)
                            .await
                            .ok()
                    })
                };

                let inner = tts_concurrency.min(8).max(1);

                let block_inputs: Vec<(usize, String, String)> = blocks
                    .iter()
                    .enumerate()
                    .map(|(idx, b)| (idx, b.text.clone(), b.pos.clone()))
                    .collect();

                let app3 = app.clone();
                let id3 = id.clone();
                let lang3 = language.clone();
                let sem3 = tts_sem.clone();
                let locks3 = tts_locks.clone();

                let block_paths: Vec<(usize, Option<String>)> = stream::iter(block_inputs)
                    .map(move |(idx, text, pos)| {
                        let app3 = app3.clone();
                        let id3 = id3.clone();
                        let lang3 = lang3.clone();
                        let sem3 = sem3.clone();
                        let locks3 = locks3.clone();

                        async move {
                            if pos == "punctuation" || text.trim().is_empty() {
                                return (idx, None);
                            }

                            let p =
                                ensure_audio_cached(app3, id3, lang3, text, "block", sem3, locks3)
                                    .await
                                    .ok();

                            (idx, p)
                        }
                    })
                    .buffer_unordered(inner)
                    .collect()
                    .await;

                for (idx, p) in block_paths {
                    blocks[idx].audio_path = p;
                }

                sentence_audio = sentence_handle.await.ok().flatten();
            }

            let sentence = Sentence {
                id: format!("{}_{}", id, i),
                original: raw.clone(),
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
        .invoke_handler(tauri::generate_handler![
            parse_text,
            save_data,
            load_data,
            delete_article_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
