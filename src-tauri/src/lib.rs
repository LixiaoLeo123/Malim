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
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordBlock {
    text: String,
    pos: String,
    definition: String,
    chinese_root: Option<String>,
    grammar_note: Option<String>,
    audio_path: Option<String>,
    // Russian-specific fields:
    lemma: Option<String>,
    gram_case: Option<u8>,       // 1-7
    gram_gender: Option<String>, // m / f / n
    gram_number: Option<String>, // sg / pl
    tense: Option<String>,       // pres / past / fut / imp / inf / gerund / ...
    aspect: Option<String>,      // impf / pf
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

// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct SentenceSplitResult {
//     sentences: Vec<String>,
// }

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
    let clean_text: String = text.nfd()
        .filter(|c| {
            let cp = *c as u32;
            !(0x0300..=0x036F).contains(&cp)
        })
        .collect();  // remove diacritics to improve TTS consistency, especially for Russian stress marks

    let mut client = connect().map_err(|e| format!("edge tts connect error: {}", e))?;

    let voice_json = format!(r#"{{"Name":"{}"}}"#, voice_name);
    let voice: EdgeVoice =
        serde_json::from_str(&voice_json).map_err(|e| format!("voice parse error: {}", e))?;

    let config = SpeechConfig::from(&voice);

    let audio = client
        .synthesize(&clean_text, &config) 
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

    let (_lang_target, task_rules, example) = if lang == "KR" {
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
You are a Russian linguistic analyzer for beginner learners.
Your goal is to make Russian fully understandable.

Return a single JSON object:

{
  "translation": "...",
  "blocks": [
    {
      "text": "...",
      "pos": "...",
      "definition": "...",
      "lemma": "...",
      "gram_case": 1-7,
      "gram_gender": "m" | "f" | "n",
      "gram_number": "sg" | "pl",
      "tense": "pres" | "past" | "fut" | "imp" | "inf" | "gerund",
      "aspect": "pf" | "impf",
      "grammar_note": "..."
    }
  ]
}

CORE PRINCIPLE:
**Context determines grammar.**
You must analyze SYNTAX (verb government, prepositions, quantifiers) to determine Case.
Do NOT rely solely on word endings.

FIELD RULES:

1. Always include:
   text, pos, definition.

2. Include grammatical fields only if meaningful.
   Do not include unused fields.

3. **Stress Marks**:
   You MUST add stress marks (acute accents) to Russian words in the `text` and `lemma` fields.
   - Mark the stressed vowel with an acute accent (e.g., "кни́га", "чита́ть").
   - Do NOT add stress to monosyllabic words (e.g., "я", "в", "на") unless necessary for disambiguation.
   - Do NOT add stress to numbers or English words.

4. **POS must be one of the following**:
   - noun
   - verb
   - adjective
   - adverb
   - pronoun
   - preposition (e.g., в, на, к, о)
   - conjunction (e.g., и, а, но)
   - particle
   - punctuation
   - unknown

5. Participles → adjective.
6. Gerunds → verb (tense = "gerund").

CATEGORY-SPECIFIC LOGIC:

**NOUNS**:
Include lemma, gram_case, gram_gender, gram_number.
**Case Logic (CRITICAL)**:
- Check the PREPOSITION. E.g., "в/на" + location = Case 6; "в/на" + motion = Case 4.
- Check the VERB. E.g., transitive verbs usually take Case 4 for direct objects.
- Check for NEGATION (e.g., "нет"). "нет" + noun = Case 2.
- Check for QUANTIFIERS. Words like "много" (many/much), "немного" (a little), "мало" (few), "сколько" (how much/many) **ALWAYS** govern Case 2 (Genitive).
  - Example: "немного та́йны" -> "та́йны" is Case 2 (Genitive Singular), NOT Nominative.

**ADJECTIVES**:
Include lemma.
**CRITICAL RESTRICTION**: Do NOT include `gram_case`, `gram_gender`, or `gram_number` for adjectives.
- Simply identify the word as an adjective and provide its base form (lemma).
- Ensure the POS is correct. If it modifies a noun, it is an adjective.

**VERBS**:
Include lemma.
Include tense if identifiable.
Include aspect (pf or impf).
**Lemma Logic**:
- The `lemma` MUST always be the **Infinitive** (the uninflected base form).
- For **Perfective verbs**, provide the Perfective Infinitive (e.g., for "напишу́", lemma is "написа́ть").
- For **Imperfective verbs**, provide the Imperfective Infinitive (e.g., for "чита́ю", lemma is "чита́ть").

**PRONOUNS (Personal)**:
Include lemma, gram_case, gram_gender, gram_number.
- For 1st/2nd person ("я", "ты"), gender defaults to "m" unless context proves otherwise.

GRAMMAR_NOTE:

Explain briefly in simple English:
- what role the word plays in the sentence
- why its form looks like this
- mention the ending change if relevant

Do NOT repeat the lemma.
Do NOT use rigid labels like "Base form:", "Why:", "How:".
Write naturally and concisely.
        "#;
        let ru_example = r#"
Example Outputs:

{
  "translation": "There is a little mystery on the table.",
  "blocks": [
    {
      "text": "На",
      "pos": "preposition",
      "definition": "on",
      "lemma": "на",
      "grammar_note": "Governs the Prepositional case when indicating location."
    },
    {
      "text": "столе́",
      "pos": "noun",
      "definition": "table",
      "lemma": "сто́л",
      "gram_case": 6,
      "gram_gender": "m",
      "gram_number": "sg",
      "grammar_note": "Prepositional case indicating location. Note the stress shifts to the ending."
    },
    {
      "text": "немно́го",
      "pos": "adverb",
      "definition": "a little",
      "lemma": "немно́го",
      "grammar_note": "Quantifier acting as an adverb, modifying the quantity of the noun."
    },
    {
      "text": "та́йны",
      "pos": "noun",
      "definition": "mystery",
      "lemma": "та́йна",
      "gram_case": 2,
      "gram_gender": "f",
      "gram_number": "sg",
      "grammar_note": "Genitive case governed by the quantifier 'немного'. The ending '-ы' is genitive singular."
    },
    {
      "text": ".",
      "pos": "punctuation",
      "definition": "."
    }
  ]
}
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

// fn create_overlapping_chunks(text: &str, chunk_size: usize, overlap_size: usize) -> Vec<String> {
//     let preliminary_sentences: Vec<&str> = text
//         .split_inclusive(|c: char| c.is_ascii_punctuation() || matches!(c, '。' | '\n'))
//         .map(|s| s.trim())
//         .filter(|s| !s.is_empty())
//         .collect();

//     if preliminary_sentences.is_empty() {
//         return Vec::new();
//     }

//     let mut chunks = Vec::new();
//     let mut current_chunk = String::new();
//     let mut current_len = 0;

//     for sentence in preliminary_sentences {
//         if current_len > 0 && current_len + sentence.len() > chunk_size {
//             chunks.push(current_chunk.clone());
//             let overlap_start_byte = current_chunk
//                 .char_indices()
//                 .rev()
//                 .take_while(|(idx, _)| current_chunk.len() - *idx <= overlap_size)
//                 .map(|(idx, _)| idx)
//                 .last()
//                 .unwrap_or(0);

//             current_chunk = current_chunk[overlap_start_byte..].to_string();
//         }
//         current_chunk.push_str(sentence);
//         current_len = current_chunk.len();
//     }

//     if !current_chunk.trim().is_empty() {
//         chunks.push(current_chunk);
//     }

//     chunks
// }

// fn create_overlapping_chunks(text: &str, chunk_size: usize, overlap_size: usize) -> Vec<String> {
//     // 1. 使用一个简单、快速的方式进行初步分割
//     let preliminary_sentences: Vec<&str> = text
//         .split_inclusive(|c: char| c.is_ascii_punctuation() || matches!(c, '。' | '\n'))
//         .map(|s| s.trim())
//         .filter(|s| !s.is_empty())
//         .collect();

//     if preliminary_sentences.is_empty() {
//         return Vec::new();
//     }

//     let mut chunks = Vec::new();
//     let mut current_chunk = String::new();
//     let mut current_len = 0;

//     // 2. 将初步句子组合成大小合适的文本块 (Chunks)
//     for sentence in preliminary_sentences {
//         if current_len > 0 && current_len + sentence.len() > chunk_size {
//             chunks.push(current_chunk.clone());
//             // 创建重叠部分
//             let overlap_start_byte = current_chunk
//                 .char_indices()
//                 .rev()
//                 .take_while(|(idx, _)| current_chunk.len() - *idx <= overlap_size)
//                 .map(|(idx, _)| idx)
//                 .last()
//                 .unwrap_or(0);

//             current_chunk = current_chunk[overlap_start_byte..].to_string();
//         }
//         current_chunk.push_str(sentence);
//         current_chunk.push(' '); // 确保句子间有空格
//         current_len = current_chunk.len();
//     }

//     // 3. 添加最后一个 chunk
//     if !current_chunk.trim().is_empty() {
//         chunks.push(current_chunk);
//     }

//     chunks
// }

// fn build_sentence_split_prompt(text_chunk: &str) -> String {
//     format!(
//         r#"You are an expert sentence boundary detector. Your task is to take a block of text and split it into a precise list of sentences.

// STRICT RULES:
// 1. Your output MUST be a single, valid JSON object.
// 2. The JSON object must contain a single key, "sentences", which is an array of strings.
// 3. Each string in the array must be a complete and distinct sentence.
// 4. Do NOT alter the content or punctuation of the sentences.
// 5. Correctly handle abbreviations (e.g., "Mr. Smith lives in the U.S.") without splitting them. A sentence must end with a terminal punctuation mark like '.', '?', '!', or be a complete thought.

// Example Input Text:
// "Hello world. This is a test... what about Mr. Jones? He lives in N.Y.C. This is the next sentence."

// Example Output JSON:
// {{
//   "sentences": [
//     "Hello world.",
//     "This is a test...",
//     "what about Mr. Jones?",
//     "He lives in N.Y.C.",
//     "This is the next sentence."
//   ]
// }}

// ---

// Now, process the following text block:
// {text_chunk}
// Output:"#
//     )
// }

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

    let mut raw_sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        current.push(c);
        if matches!(c, '.' | '。' | '!' | '?' | '\n') {
            while let Some(&next_c) = chars.peek() {
                if matches!(next_c, '.' | '。' | '!' | '?' | '\n') {
                    current.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let trimmed = current.trim();
            if !trimmed.is_empty() {
                raw_sentences.push(trimmed.to_string());
            }
            current.clear();
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        raw_sentences.push(trimmed.to_string());
    }

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
            let (mut blocks, translation) = if cached
                .as_ref()
                .map_or(false, |b| !b.translation.starts_with('$'))
            {
                let b = cached.unwrap();
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
                            lemma: None,
                            gram_case: None,
                            gram_gender: None,
                            gram_number: None,
                            tense: None,
                            aspect: None,
                        }],
                        "$Translation unavailable due to error.".to_string(),
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
