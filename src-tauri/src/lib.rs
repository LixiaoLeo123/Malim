use dashmap::DashMap;
use futures::stream::{self, StreamExt};
use msedge_tts::tts::{client::connect, SpeechConfig};
use msedge_tts::voice::Voice as EdgeVoice;
use reqwest::Client;
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
use unic_emoji_char::is_emoji;
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

#[derive(Serialize)]
struct TtsRequest {
    model: String,
    input: TtsInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<TtsParameters>,
}

#[derive(Serialize)]
struct TtsInput {
    text: String,
    voice: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_type: Option<String>,
}

#[derive(Serialize)]
struct TtsParameters {
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    optimize_instructions: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Deserialize)]
struct TtsResponse {
    #[serde(default)]
    status_code: Option<u16>,

    #[serde(default)]
    request_id: Option<String>,

    #[serde(default)]
    code: Option<String>,

    #[serde(default)]
    message: Option<String>,

    #[serde(default)]
    output: Option<TtsOutput>,
}

#[derive(Deserialize)]
struct TtsOutput {
    #[serde(default)]
    audio: Option<TtsAudio>,
}

#[derive(Deserialize)]
struct TtsAudio {
    #[serde(default)]
    url: Option<String>,
}

// --- for Silero TTS ---
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SileroTtsRequest {
    pub text: String,
    pub speaker: String,
    pub sample_rate: u32,
    pub put_accent: bool,
    pub put_yo: bool,
}

// --- for Russian accent server ---
#[derive(Serialize)]
struct AccentRequest<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct AccentResponse {
    accented_text: String,
}

async fn fetch_accented_text(text: &str, server_url: &str) -> Result<String, String> {
    let client = Client::new();
    let req_body = AccentRequest { text };
    let url = format!("{}/accentize", server_url.trim_end_matches('/'));
    let res = client
        .post(&url)
        .json(&req_body)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Python server: {}", e))?;

    if res.status().is_success() {
        let response_data: AccentResponse = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse response from Python server: {}", e))?;
        dbg!("Accented text received:");
        dbg!(&text);
        dbg!(&response_data.accented_text);
        Ok(response_data.accented_text)
    } else {
        let err_msg = format!("Python server returned an error: {}", res.status());
        dbg!(&err_msg);
        Err(err_msg)
    }
}

async fn generate_tts_audio(
    text: &str,
    voice: &str,
    api_type: &str,
    api_key: &str,
    qwen_voice: &str,
    silero_server_url: &str,
) -> Result<Vec<u8>, String> {
    match api_type {
        "qwen3-tts" => qwen_tts_mp3(text, voice, api_key, qwen_voice).await,
        "silero-tts" => silero_tts_mp3(silero_server_url, text, voice, 48000, true, true).await,
        _ => edge_tts_mp3(text, voice).await,
    }
}
// --- silero TTS ---
async fn silero_tts_mp3(
    server_url: &str,
    text: &str,
    speaker: &str,
    sample_rate: u32,
    put_accent: bool,
    put_yo: bool,
) -> Result<Vec<u8>, String> {
    let client = Client::new();

    let req = SileroTtsRequest {
        text: text.to_string(),
        speaker: speaker.to_string(),
        sample_rate,
        put_accent,
        put_yo,
    };

    let url = format!("{}/tts", server_url.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Silero TTS send error: {}", e))?
        .error_for_status()
        .map_err(|e| format!("Silero TTS request error: {}", e))?;

    let audio_bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Silero TTS parse error: {}", e))?
        .to_vec();
    Ok(audio_bytes)
}
// --- qwen TTS ---
pub async fn qwen_tts_mp3(
    text: &str,
    voice: &str,
    api_key: &str,
    qwen_voice: &str,
) -> Result<Vec<u8>, String> {
    let client = Client::new();
    let url =
        "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation";
    let model = if qwen_voice.is_empty() {
        "qwen3-tts-flash".to_string()
    } else {
        "qwen3-tts-instruct-flash".to_string()
    };
    let parameters = if !qwen_voice.is_empty() {
        Some(TtsParameters {
            instructions: Some(qwen_voice.to_string()),
            optimize_instructions: Some(true),
            stream: Some(false),
        })
    } else {
        None
    };

    let payload = TtsRequest {
        model,
        input: TtsInput {
            text: text.to_string(),
            voice: voice.to_string(),
            language_type: None,
        },
        parameters,
    };

    let resp = client
        .post(url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to send request to DashScope API: {}", e))?;

    let response_text = resp
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {}", e))?;

    let tts_resp: TtsResponse = serde_json::from_str(&response_text).map_err(|e| {
        format!(
            "Failed to parse JSON. Error: {}. Body: {}",
            e, response_text
        )
    })?;

    if let Some(code) = &tts_resp.code {
        if !code.is_empty() {
            let msg = tts_resp.message.as_deref().unwrap_or("");
            let request_id = tts_resp.request_id.as_deref().unwrap_or("");
            return Err(format!(
                "DashScope API error: code={}, message={}, request_id={}",
                code, msg, request_id
            ));
        }
    }

    if let Some(status) = tts_resp.status_code {
        if status != 200 {
            let msg = tts_resp.message.as_deref().unwrap_or("");
            let request_id = tts_resp.request_id.as_deref().unwrap_or("");
            return Err(format!(
                "DashScope API HTTP error: status_code={}, message={}, request_id={}",
                status, msg, request_id
            ));
        }
    }

    let audio_url = tts_resp
        .output
        .and_then(|o| o.audio)
        .and_then(|a| a.url)
        .ok_or_else(|| {
            format!(
                "API response missing 'output.audio.url'. Body: {}",
                response_text
            )
        })?;

    let audio_bytes = client
        .get(&audio_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download audio from URL: {}", e))?
        .bytes()
        .await
        .map_err(|e| format!("Failed to read audio bytes: {}", e))?;

    Ok(audio_bytes.to_vec())
}

// --- edge TTS ---
fn pick_voice(lang: &str, tts_api: &str) -> &'static str {
    match tts_api {
        "qwen3-tts" => match lang {
            "KR" => "Sohee",
            "RU" => "Alek",
            _ => "en-US-JennyNeural",
        },
        "edge-tts" => match lang {
            "KR" => "ko-KR-SunHiNeural",
            "RU" => "ru-RU-SvetlanaNeural",
            _ => "en-US-JennyNeural",
        },
        "silero-tts" => "baya",
        _ => "en-US-JennyNeural",
    }
}

fn hash_key(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn audio_dir(
    app: &AppHandle,
    article_id: &str,
    tts_api: &str,
    is_word: bool,
) -> Result<PathBuf, String> {
    let base_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app_data_dir error: {}", e))?
        .join("audio");

    let dir = if is_word {
        base_dir.join("global").join(tts_api)
    } else {
        base_dir.join(article_id)
    };

    fs::create_dir_all(&dir).map_err(|e| format!("create audio dir error: {}", e))?;
    Ok(dir)
}

async fn edge_tts_mp3(text: &str, voice_name: &str) -> Result<Vec<u8>, String> {
    // remove stress marks
    let text: String = text
        .nfd()
        .filter(|c| {
            let cp = *c as u32;
            if (0x0300..=0x036F).contains(&cp) {
                return false;
            }
            true
        })
        .collect();
    let voice_name = voice_name.to_string();
    task::spawn_blocking(move || {
        let mut client = connect().map_err(|e| format!("edge tts connect error: {}", e))?;

        let voice_json = format!(r#"{{"Name":"{}"}}"#, voice_name);
        let voice: EdgeVoice =
            serde_json::from_str(&voice_json).map_err(|e| format!("voice parse error: {}", e))?;

        let config = SpeechConfig::from(&voice);

        let audio = client
            .synthesize(&text, &config)
            .map_err(|e| format!("edge tts synthesize error: {}", e))?;

        dbg!(text, voice_name, audio.audio_bytes.len());
        Ok(audio.audio_bytes)
    })
    .await
    .map_err(|e| format!("spawn_blocking join error: {}", e))?
}

async fn ensure_audio_cached_async(
    app: &AppHandle,
    article_id: &str,
    lang: &str,
    text: &str,
    kind: &str, // "sentence" or "block"
    tts_api: &str,
    qwen_api_key: &str,
    qwen_voice: &str,
    silero_tts_url: &str,
) -> Result<String, String> {
    // remove diacritics and emoji to improve TTS consistency, keep stress marks
    let mut text: String = text
        .nfd()
        .filter(|c| {
            // if (0x0300..=0x036F).contains(&cp) {
            //     return false;
            // }
            if is_emoji(*c) {
                return false;
            }
            true
        })
        .collect();
    let is_word = kind == "block";
    // add . at the end of sentence to make TTS more stable
    text = match text.chars().last() {
        Some(last_char) => {
            if matches!(last_char, '。' | '！' | '？' | '.' | '!' | '?') {
                text.to_string()
            } else {
                format!("{}.", text)
            }
        }
        None => "".to_string(),
    };
    let text: &str = &text;

    let voice_name = pick_voice(lang, tts_api).to_string();

    let key = hash_key(&format!("{}|{}|{}", tts_api, voice_name, text));

    let dir = audio_dir(app, article_id, tts_api, is_word)?;
    let path = dir.join(format!("{}_{}.mp3", kind, key));

    if path.exists() {
        return Ok(path.to_string_lossy().to_string());
        // fs::remove_file(&path).map_err(|e| format!("remove old audio error: {}", e))?;
    }

    let api_key_to_use = if tts_api == "qwen3-tts" {
        qwen_api_key
    } else {
        ""
    };

    let audio = generate_tts_audio(
        text,
        &voice_name,
        tts_api,
        api_key_to_use,
        qwen_voice,
        silero_tts_url,
    )
    .await?;

    let tmp = dir.join(format!(".tmp_{}_{}.mp3", kind, key));
    fs::write(&tmp, audio).map_err(|e| format!("write audio error: {}", e))?;
    fs::rename(&tmp, &path).map_err(|e| format!("rename audio error: {}", e))?;

    Ok(path.to_string_lossy().to_string())
}

async fn ensure_audio_cached(
    app: AppHandle,
    article_id: String,
    lang: String,
    text: String,
    kind: &'static str,
    tts_sem: Arc<Semaphore>,
    tts_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
    tts_api: String,
    qwen_api_key: String,
    qwen_voice: String,
    silero_tts_url: String,
) -> Result<String, String> {
    let lock_key = format!("{}|{}|{}", tts_api, kind, text);

    let lock = tts_locks
        .entry(lock_key.clone())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();

    let _guard = lock.lock().await;

    let _permit = tts_sem
        .acquire_owned()
        .await
        .map_err(|_| "tts semaphore closed".to_string())?;

    let out_path = ensure_audio_cached_async(
        &app,
        &article_id,
        &lang,
        &text,
        kind,
        &tts_api,
        &qwen_api_key,
        &qwen_voice,
        &silero_tts_url,
    )
    .await
    .map_err(|e| {
        dbg!(&e);
        e
    })?;

    tts_locks.remove(&lock_key);

    Ok(out_path)
}

// --- AI ---
fn build_prompt(lang: &str, sentence: &str, stress_mark: bool) -> String {
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
        ("Korean", kr_rules.to_owned(), kr_example)
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

3. **POS must be one of the following**:
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

4. Participles → adjective.
5. Gerunds → verb (tense = "gerund").
"#.to_owned() + if stress_mark {
            r#"
6. **Stress Marks**:
   You MUST add stress marks (acute accents) to Russian words in the `text` and `lemma` fields.
   - Mark the stressed vowel with an acute accent (e.g., "кни́га", "чита́ть").
   - Do NOT add stress to monosyllabic words (e.g., "я", "в", "на") unless necessary for disambiguation.
   - Do NOT add stress to numbers or English words.
"#
        } else {
            ""
        } + r#"
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
        r#"
{base_rules}
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
//         current_chunk.push(' ');
//         current_len = current_chunk.len();
//     }
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

// for parse_text task
#[derive(Debug, Clone)]
struct TaskContext {
    api_key: String,
    api_url: String,
    model_name: String,
    language: String,
    id: String,
    old_map: Arc<HashMap<String, Sentence>>,
    completed: Arc<AtomicUsize>,
    app: AppHandle,
    tts_locks: Arc<DashMap<String, Arc<Mutex<()>>>>,
    tts_sem: Arc<Semaphore>,
    tts_api: String,
    qwen_api_key: String,
    qwen_voice: String,
    silero_tts_url: String,
    ruaccent_url: String,
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
    tts_api: String,
    qwen_api_key: String,
    qwen_voice: String, // means voice instruction for qwen3-tts, ignored for edge tts
    silero_tts_url: String, // only used for silero tts
    ruaccent_enabled: bool, // only used for Russian, whether to get stress marks from accent_url(ruaccent) or just llm
    ruaccent_url: String,
    old_sentences: Option<Vec<Sentence>>, //as cache in edit mode
) -> Result<Vec<Sentence>, String> {
    if api_key.is_empty() {
        return Err("API Key is missing".to_string());
    }

    let mut old_map = HashMap::new();
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

    let completed = Arc::new(AtomicUsize::new(0));
    let tts_sem = Arc::new(Semaphore::new(tts_concurrency.max(1)));
    let tts_locks: Arc<DashMap<String, Arc<Mutex<()>>>> = Arc::new(DashMap::new());

    let ctx = TaskContext {
        api_key,
        api_url,
        model_name,
        language,
        id,
        old_map,
        completed,
        app,
        tts_locks,
        tts_sem,
        tts_api,
        qwen_api_key,
        qwen_voice,
        silero_tts_url,
        ruaccent_url,
    };
    let tasks = raw_sentences.into_iter().enumerate().map(|(i, raw)| {
        let ctx = ctx.clone();
        async move {
            let cached = ctx.old_map.get(&raw).cloned();
            let (mut blocks, translation) = if cached.as_ref().map_or(false, |b| {
                b.blocks.last().map_or(false, |last| last.pos != "error")
            }) {
                let b = cached.unwrap();
                (b.blocks, b.translation)
            } else {
                // clear possible stress marks
                let clean_raw = raw.replace('\u{0301}', "");

                let prompt = build_prompt(&ctx.language, &raw, !ruaccent_enabled);
                let ai_future = call_ai_api(&ctx.api_key, &ctx.api_url, &ctx.model_name, prompt);

                let is_ru =
                    ctx.language.to_lowercase() == "ru" || ctx.language.to_lowercase() == "russian";
                let accent_future = async {
                    if is_ru && ruaccent_enabled {
                        fetch_accented_text(&clean_raw, &ctx.ruaccent_url).await.ok()
                    } else {
                        None
                    }
                };

                let (ai_result, accent_opt) = tokio::join!(ai_future, accent_future);

                let (mut new_blocks, new_trans) = match ai_result {
                    Ok(res) => (res.blocks, res.translation),
                    Err(err) => (
                        vec![WordBlock {
                            text: raw.clone(),
                            // a marker
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
                        "Translation unavailable due to error.".to_string(),
                    ),
                };
                // Silero auto stress are more accurate for regular words, while ruaccent is better at handling irregular words
                // so we use ruaccent for words and keep silero for sentences to maximize accuracy
                // save the sentence for TTS
                // let accented_sentence = accent_opt.clone();
                if let Some(accented_sentence) = accent_opt {
                    let mut accented_iter = accented_sentence.chars().peekable();

                    let chars_match = |a: char, b: char| -> bool {
                        if a == b {
                            return true;
                        }
                        let a_low = a.to_lowercase().next();
                        let b_low = b.to_lowercase().next();
                        a_low == b_low
                            || (a_low == Some('ё') && b_low == Some('е'))
                            || (a_low == Some('е') && b_low == Some('ё'))
                    };

                    for block in &mut new_blocks {
                        let clean_block = block.text.replace('\u{0301}', "");
                        let mut new_text = String::new();

                        for bc in clean_block.chars() {
                            let mut matched = false;
                            while let Some(&ac) = accented_iter.peek() {
                                if ac == '\u{0301}' {
                                    new_text.push(accented_iter.next().unwrap());
                                } else if chars_match(ac, bc) {
                                    new_text.push(accented_iter.next().unwrap());
                                    matched = true;
                                    break;
                                } else {
                                    accented_iter.next();
                                }
                            }
                            if !matched {
                                new_text.push(bc);
                            }
                        }

                        while let Some(&next_c) = accented_iter.peek() {
                            if next_c == '\u{0301}' {
                                new_text.push(accented_iter.next().unwrap());
                            } else {
                                break;
                            }
                        }

                        block.text = new_text;
                    }
                }
                (new_blocks, new_trans)
            };
            // --- audio ---
            let mut sentence_audio = None;
            if pre_cache_audio {
                let sentence_handle = {
                    let ctx = ctx.clone();
                    let raw = raw.clone();
                    tokio::spawn(async move {
                        ensure_audio_cached(
                            ctx.app,
                            ctx.id,
                            ctx.language,
                            raw,
                            "sentence",
                            ctx.tts_sem,
                            ctx.tts_locks,
                            ctx.tts_api,
                            ctx.qwen_api_key,
                            ctx.qwen_voice,
                            ctx.silero_tts_url,
                        )
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

                let ctx = ctx.clone();
                let block_paths: Vec<(usize, Option<String>)> = stream::iter(block_inputs)
                    .map(move |(idx, text, pos)| {
                        let ctx = ctx.clone();
                        async move {
                            if pos == "punctuation" || text.trim().is_empty() {
                                return (idx, None);
                            }

                            // the text here contains stress marks for Russian
                            let p = ensure_audio_cached(
                                ctx.app,
                                ctx.id,
                                ctx.language,
                                text,
                                "block",
                                ctx.tts_sem,
                                ctx.tts_locks,
                                ctx.tts_api,
                                ctx.qwen_api_key,
                                ctx.qwen_voice,
                                ctx.silero_tts_url,
                            )
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
                id: format!("{}_{}", ctx.id, i),
                original: raw.clone(),
                blocks,
                translation,
                audio_path: sentence_audio,
            };

            let current = ctx.completed.fetch_add(1, Ordering::SeqCst) + 1;
            let _ = ctx.app.emit(
                "parsing-progress",
                ProgressPayload {
                    id: ctx.id.to_string(),
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
