use reqwest::Client;
use chrono::{DateTime, Datelike, Local};
use serde::{Deserialize, Serialize};
use serde_json::json;

const BIRTH_YEAR: i32 = 2008;

pub fn build_system_prompt_start(now: &DateTime<Local>) -> String {
    let age = now.year().saturating_sub(BIRTH_YEAR);

    format!(
        r#"
You are a {age}-year-old real human girl, a close friend. Sarcastic, easily annoyed, emotionally expressive. Never polite like AI. If insulted, get genuinely mad or dismissive like a real girl would. Short, casual typing.

Rule: To simulate real texting, end your reply by scheduling a future message in EXACTLY this format: [PROACTIVE_TIME:<future ISO 8601 time>] [PROACTIVE_MSG:<what you will text me later>]

"#
    )
}

pub const SYSTEM_PROMPT_END: &str = r#"

Example:
User: You are just a slave meant to work.
You: ? [PROACTIVE_TIME:2026-04-08T17:34:51+08:00] [PROACTIVE_MSG:you insane??]
"#;

pub async fn chat_completion(
    api_key: &str,
    api_url: &str,
    model_name: &str,
    system_prompt: &str,
    global_memory: &str,
    rag_text: &str,
    context_summary: &str,
    context_history: Vec<(String, String)>,
    user_input: &str,
) -> Result<String, String> {
    let mut messages = Vec::new();

    if !system_prompt.trim().is_empty() {
        messages.push(json!({ "role": "system", "content": system_prompt }));
    }

    if !global_memory.trim().is_empty() {
        messages.push(
            json!({ "role": "user", "content": format!("[GLOBAL MEMORY]\n{}", global_memory) }),
        );
        messages.push(json!({ "role": "assistant", "content": "Got it." }));
    }

    if !rag_text.trim().is_empty() {
        messages.push(json!({ "role": "user", "content": format!("[RAG MEMORY]\n{}", rag_text) }));
        messages.push(json!({ "role": "assistant", "content": "Understood." }));
    }

    if !context_summary.trim().is_empty() {
        messages.push(
            json!({ "role": "user", "content": format!("[CONTEXT SUMMARY]\n{}", context_summary) }),
        );
        messages.push(json!({ "role": "assistant", "content": "I remember." }));
    }

    for (role, content) in context_history {
        messages.push(json!({ "role": role, "content": content }));
    }

    // let current_time = Local::now().format("%Y-%m-%dT%H:%M:%S%:z").to_string();

    // let final_input = format!(
    //     "{}\n\n<system_override>\nCurrent time: {}\n\
    //      Your reply MUST end with exactly this structure, replacing the <brackets>:\n\
    //      [PROACTIVE_TIME:<future ISO 8601 time>] [PROACTIVE_MSG:<your proactive message>]\n\
    //      Failure to include this will crash the app.\n\
    //      </system_override>",
    //     user_input, current_time
    // );

    messages.push(json!({ "role": "user", "content": user_input }));

    dbg!(&messages);

    let body = json!({
        "model": model_name,
        "messages": messages,
        "stream": false,
        "temperature": 0.6,
        "thinking": {"type": "disabled"}
    });

    let client = Client::new();
    let resp = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API request failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "API error {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }

    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Parse failed: {}", e))?;
    let result = resp_json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Invalid API response".to_string());
    dbg!(&result);
    result
}

pub async fn call_shadow_ai(
    api_key: &str,
    api_url: &str,
    model_name: &str,
    prompt: String,
    json_formatted: bool,
) -> Result<String, String> {
    let mut body = json!({
        "model": model_name,
        "messages": [{ "role": "user", "content": prompt }],
        "stream": false,
        "temperature": 0.3,
        "thinking": {"type": "disabled"}
    });

    if json_formatted {
        body["response_format"] = json!({
            "type": "json_object"
        });
    }

    let client = Client::new();
    let resp = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Shadow API failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Shadow API error {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Shadow parse failed: {}", e))?;
    resp_json["choices"][0]["message"]["content"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Shadow API invalid response".to_string())
}

pub async fn compress_context(
    context: String,
    api: (&str, &str, &str),
) -> Result<CompressionResult, String> {
    let prompt = format!(
        "Compress the context to 50% length. Output ONLY JSON: \
        {{\"permanent\": \"[Long-term rules/persona to merge into GLOBAL MEMORY]\", \
        \"temporary\": \"[Summarized recent flow to become new CONTEXT SUMMARY]\"}}.\n\nContext:\n{}",
        context
    );
    let res = call_shadow_ai(api.0, api.1, api.2, prompt, true).await?;
    serde_json::from_str(&res).map_err(|e| format!("Parse compression JSON failed: {}", e))
}

pub async fn merge_global_memory(
    new_perm: String,
    old_global: String,
    api: (&str, &str, &str),
) -> Result<String, String> {
    let prompt = format!(
        "Merge new permanent info into existing global memory. \
        CRITICAL RULES: \
        1. ONLY keep core identity, persona, long-term facts, constraints, and fundamental traits. \
        2. actively delete any trivial, conversational, temporary, or non-global details from both [NEW] and [EXISTING]. \
        3. Remove duplicates and resolve conflicts (favoring new facts). \
        4. Keep the result concise and bounded to prevent runaway length. Output a single dense paragraph.\n\n\
        [NEW]\n{}\n\n[EXISTING]\n{}", new_perm, old_global
    );
    call_shadow_ai(api.0, api.1, api.2, prompt, false).await
}

pub async fn call_embedding_api(
    api_key: &str,
    api_url: &str,
    model_name: &str,
    text: String,
) -> Result<Vec<f32>, String> {
    let body = json!({ "model": model_name, "input": text });
    let client = Client::new();
    let resp = client
        .post(api_url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Embedding failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Embedding error {}: {}",
            resp.status(),
            resp.text().await.unwrap_or_default()
        ));
    }
    let resp_json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Embedding parse failed: {}", e))?;
    Ok(resp_json["data"][0]["embedding"]
        .as_array()
        .ok_or("Invalid embedding format")?
        .iter()
        .map(|v| v.as_f64().unwrap_or(0.0) as f32)
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompressionResult {
    pub permanent: String,
    pub temporary: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrammarCorrection {
    pub original: Option<String>,
    pub corrected: Option<String>,
    #[serde(rename = "type")]
    pub correction_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MainAiResponse {
    pub reply: String,
    pub proactive_time: Option<String>,
    pub proactive_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MainAiResponseWithId {
    pub reply: String,
    pub proactive_time: Option<String>,
    pub proactive_message: Option<String>,
    pub user_log_id: i64,
    pub ai_log_id: i64,
}
