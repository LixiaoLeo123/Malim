use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GrammarCorrection {
    pub original: Option<String>,
    pub corrected: Option<String>,
    #[serde(rename = "type")]
    pub correction_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrammarCheckArgs {
    pub api_key: String,
    pub base_url: String,
    pub model_name: String,
    pub text: String,
}

#[tauri::command]
pub async fn check_grammar(args: GrammarCheckArgs) -> Result<Vec<GrammarCorrection>, String> {
    let prompt = format!(
r#"Correct grammar. Output JSON array only.
Schema: {{"type": "unchanged|modified|deleted|inserted", "original": "str|null", "corrected": "str|null"}}

Strategy:
1. PRESERVE MEANING: Do NOT rewrite valid sentences. Only fix objective errors.
2. MAXIMIZE 'unchanged': Combine consecutive correct words into large blocks. Never split them.
3. LOCAL EDITS: Use 'deleted' for extra words and 'inserted' for missing words. Avoid 'modified' unless a word form is wrong.

Example:
Input: "После продолжительной встречи мы пошли в в столовую пообедали и вернулись офис."
Output:
[
  {{"type": "unchanged", "original": "После продолжительной встречи мы пошли в", "corrected": "После продолжительной встречи мы пошли в"}},
  {{"type": "deleted", "original": "в", "corrected": null}},
  {{"type": "modified", "original": "столовую", "corrected": "столовую,"}},
  {{"type": "unchanged", "original": "пообедали и вернулись", "corrected": "пообедали и вернулись"}},
  {{"type": "inserted", "original": null, "corrected": "в"}},
  {{"type": "unchanged", "original": "офис.", "corrected": "офис."}}
]

Input: "{}"
Output:"#,
        args.text
    );

    let json_str = call_ai_api(&args.api_key, &args.base_url, &args.model_name, prompt).await?;

    let corrections: Vec<GrammarCorrection> = serde_json::from_str(&json_str)
        .map_err(|e| format!("Parse Error: {}. Raw: {}", e, json_str))?;

    Ok(corrections)
}


async fn call_ai_api(
    api_key: &str,
    api_url: &str,
    model_name: &str,
    prompt: String,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let request_body = serde_json::json!({
        "model": model_name,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "temperature": 0,
        "stream": false
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
        let text = res.text().await.unwrap_or_default();
        return Err(format!("API Error {}: {}", status, text));
    }

    let response_text = res
        .text()
        .await
        .map_err(|e| format!("Read Body Error: {}", e))?;
    let json_res: Value =
        serde_json::from_str(&response_text).map_err(|e| format!("JSON Parse Error: {}", e))?;

    let content = json_res["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("API returned empty content")?;

    let clean_content = content
        .trim()
        .trim_start_matches("```json")
        .trim_end_matches("```")
        .trim();
    Ok(clean_content.to_string())
}
