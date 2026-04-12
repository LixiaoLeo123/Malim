use serde::{Deserialize, Serialize};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};

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

pub fn diff_to_corrections(original: &str, corrected: &str) -> Vec<GrammarCorrection> {
    let diff = TextDiff::from_words(original, corrected);

    let mut corrections = Vec::new();
    let mut last_unchanged_index: Option<usize> = None;

    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Equal => {
                let value = change.to_string_lossy().to_string();
                let trimmed_value = value.trim();
                if trimmed_value.is_empty() {
                    continue;
                }

                if let Some(last_idx) = last_unchanged_index {
                    let last_correction: &mut GrammarCorrection = &mut corrections[last_idx];
                    last_correction.original = Some(format!(
                        "{} {}",
                        last_correction.original.as_ref().unwrap(),
                        trimmed_value
                    ));
                    last_correction.corrected = Some(format!(
                        "{} {}",
                        last_correction.corrected.as_ref().unwrap(),
                        trimmed_value
                    ));
                } else {
                    last_unchanged_index = Some(corrections.len());
                    corrections.push(GrammarCorrection {
                        correction_type: "unchanged".to_string(),
                        original: Some(trimmed_value.to_string()),
                        corrected: Some(trimmed_value.to_string()),
                    });
                }
            }
            ChangeTag::Delete => {
                last_unchanged_index = None;
                let value = change.to_string_lossy().trim().to_string();
                if !value.is_empty() {
                    corrections.push(GrammarCorrection {
                        correction_type: "deleted".to_string(),
                        original: Some(value),
                        corrected: None,
                    });
                }
            }
            ChangeTag::Insert => {
                last_unchanged_index = None; // 加上这一行，切断与前面 unchanged 的合并
                let value = change.to_string_lossy().trim().to_string();
                if !value.is_empty() {
                    corrections.push(GrammarCorrection {
                        correction_type: "inserted".to_string(),
                        original: None,
                        corrected: Some(value),
                    });
                }
            }
        }
    }

    corrections
}


#[tauri::command]
pub async fn check_grammar(args: GrammarCheckArgs) -> Result<Vec<GrammarCorrection>, String> {
    let prompt = format!(
        r#"You are a grammar correction engine. Fix objective grammatical errors in the following text.
RULES:
1. PRESERVE MEANING: Do NOT rewrite for style. Only fix objective errors (spelling, punctuation, syntax).
2. MINIMIZE CHANGES: Keep unchanged parts EXACTLY identical to the original, including spaces and line breaks.
3. OUTPUT: Output the corrected text ONLY. Do NOT output JSON, explanations, or quotes.

Original:
{}

Corrected:"#,
        args.text
    );

    let corrected_text = call_ai_api(&args.api_key, &args.base_url, &args.model_name, prompt)
        .await?
        .trim()
        .to_string();
    let corrections = diff_to_corrections(&args.text, &corrected_text);

    Ok(corrections)
}

// #[tauri::command]
// pub async fn check_grammar(args: GrammarCheckArgs) -> Result<Vec<GrammarCorrection>, String> {
//     let prompt = format!(
// r#"Correct grammar. Output JSON array only.
// Schema: {{"type": "unchanged|modified|deleted|inserted", "original": "str|null", "corrected": "str|null"}}

// Strategy:
// 1. PRESERVE MEANING: Do NOT rewrite valid sentences. Only fix objective errors.
// 2. MAXIMIZE 'unchanged': Combine consecutive correct words into large blocks. Never split them.
// 3. LOCAL EDITS: Use 'deleted' for extra words and 'inserted' for missing words. Avoid 'modified' unless a word form is wrong.

// Example:
// Input: "После продолжительной встречи мы пошли в в столовую пообедали и вернулись офис."
// Output:
// [
//   {{"type": "unchanged", "original": "После продолжительной встречи мы пошли в", "corrected": "После продолжительной встречи мы пошли в"}},
//   {{"type": "deleted", "original": "в", "corrected": null}},
//   {{"type": "modified", "original": "столовую", "corrected": "столовую,"}},
//   {{"type": "unchanged", "original": "пообедали и вернулись", "corrected": "пообедали и вернулись"}},
//   {{"type": "inserted", "original": null, "corrected": "в"}},
//   {{"type": "unchanged", "original": "офис.", "corrected": "офис."}}
// ]

// Input: "{}"
// Output:"#,
//         args.text
//     );

//     let json_str = call_ai_api(&args.api_key, &args.base_url, &args.model_name, prompt).await?;

//     let corrections: Vec<GrammarCorrection> = serde_json::from_str(&json_str)
//         .map_err(|e| format!("Parse Error: {}. Raw: {}", e, json_str))?;

//     Ok(corrections)
// }

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
