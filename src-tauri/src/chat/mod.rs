pub mod ai;
pub mod commands;
pub mod db;
pub mod token;
pub mod vector;

use ai::{
    call_embedding_api, call_shadow_ai, chat_completion, compress_context, merge_global_memory,
    GrammarCorrection, MainAiResponse, MainAiResponseWithId, SYSTEM_PROMPT_END,
    SYSTEM_PROMPT_START,
};
use chrono::Local;
use db::DbState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use token::count_tokens;
use tokio::sync::Mutex;
use vector::cosine_similarity;

const DEFAULT_MAX_TOTAL_TOKENS: usize = 4000;
const DEFAULT_MAX_RAG_TOKENS: usize = 1000;
const DEFAULT_MAX_RAG_APPEND_TOKENS: usize = 1000; // token used to append new content with previous rag
const DEFAULT_MAX_USER_TOKENS: usize = 500;

#[derive(Debug, Clone, Copy)]
pub struct TokenLimits {
    pub max_total_tokens: usize,
    pub max_rag_tokens: usize,
    pub max_rag_append_tokens: usize,
    pub max_user_tokens: usize,
}

impl Default for TokenLimits {
    fn default() -> Self {
        Self {
            max_total_tokens: DEFAULT_MAX_TOTAL_TOKENS,
            max_rag_tokens: DEFAULT_MAX_RAG_TOKENS,
            max_rag_append_tokens: DEFAULT_MAX_RAG_APPEND_TOKENS,
            max_user_tokens: DEFAULT_MAX_USER_TOKENS,
        }
    }
}

impl TokenLimits {
    pub fn from_input(
        max_total_tokens: Option<u32>,
        max_rag_tokens: Option<u32>,
        max_rag_append_tokens: Option<u32>,
        max_user_tokens: Option<u32>,
    ) -> Self {
        let defaults = Self::default();

        // Keep limits in a safe range
        let max_total = max_total_tokens
            .map(|v| v as usize)
            .unwrap_or(defaults.max_total_tokens)
            .clamp(500, 32000);

        let mut max_rag = max_rag_tokens
            .map(|v| v as usize)
            .unwrap_or(defaults.max_rag_tokens)
            .clamp(100, max_total);

        let max_rag_append = max_rag_append_tokens
            .map(|v| v as usize)
            .unwrap_or(defaults.max_rag_append_tokens)
            .clamp(100, max_total);

        let mut max_user = max_user_tokens
            .map(|v| v as usize)
            .unwrap_or(defaults.max_user_tokens)
            .clamp(50, max_total);

        if max_rag > max_total {
            max_rag = max_total;
        }
        if max_user > max_total {
            max_user = max_total;
        }
        Self {
            max_total_tokens: max_total,
            max_rag_tokens: max_rag,
            max_rag_append_tokens: max_rag_append,
            max_user_tokens: max_user,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub grammar_corrections: Option<String>, // JSON string of Vec<GrammarCorrection>
    pub parsed_content: Option<String>,      // JSON string of parsed content for grammar notes
}

#[derive(Serialize)]
pub struct ChatLogsResponse {
    pub messages: Vec<ChatMessage>,
    pub has_more: bool, // ealier
}

pub struct MemoryHandler {
    pub db: Arc<Mutex<DbState>>,
}

impl MemoryHandler {
    pub fn new(db_path: &str) -> Result<Self, String> {
        Ok(Self {
            db: Arc::new(Mutex::new(DbState::new(db_path)?)),
        })
    }

    fn parse_context(context: &str) -> (String, Vec<(String, String)>) {
        let summary: String;
        let mut history = Vec::new();

        if let Some(idx) = context.find("[History]") {
            summary = context[..idx].replace("[Summary]", "").trim().to_string();
            let hist_str = &context[idx + 9..];
            let mut current_role = String::new();
            let mut current_content = String::new();

            for line in hist_str.lines() {
                if line.starts_with("User: ") {
                    if !current_role.is_empty() {
                        history.push((current_role, current_content.trim().to_string()));
                    }
                    current_role = "user".to_string();
                    current_content = line[6..].to_string();
                } else if line.starts_with("Assistant: ") {
                    if !current_role.is_empty() {
                        history.push((current_role, current_content.trim().to_string()));
                    }
                    current_role = "assistant".to_string();
                    current_content = line[11..].to_string();
                } else if !current_role.is_empty() {
                    current_content.push_str(&format!("\n{}", line));
                }
            }
            if !current_role.is_empty() {
                history.push((current_role, current_content.trim().to_string()));
            }
        } else {
            summary = context.replace("[Summary]", "").trim().to_string();
        }
        (summary, history)
    }

    fn append_to_context(context: &str, user_input: &str, ai_reply: &str) -> String {
        if context.contains("[History]") {
            format!(
                "{}\nUser: {}\nAssistant: {}",
                context.trim_end(),
                user_input,
                ai_reply
            )
        } else {
            format!(
                "{}\n[History]\nUser: {}\nAssistant: {}",
                context.trim_end(),
                user_input,
                ai_reply
            )
        }
    }

    pub async fn update_parsed_content(
        &self,
        log_id: i64,
        parsed_content: String,
    ) -> Result<(), String> {
        let db_lock = self.db.lock().await;
        db_lock.update_parsed_content(log_id, parsed_content)
    }

    pub async fn save_grammar(
        &self,
        log_id: i64,
        corrections: Vec<GrammarCorrection>,
    ) -> Result<(), String> {
        let json = serde_json::to_string(&corrections).map_err(|e| e.to_string())?;
        let db_lock = self.db.lock().await;
        db_lock.update_grammar(log_id, json)
    }

    pub async fn handle_message(
        &self,
        user_input: String,
        main_api: (&str, &str, &str),
        shadow_api: (&str, &str, &str),
        embed_api: (&str, &str, &str),
        token_limits: TokenLimits,
    ) -> Result<MainAiResponseWithId, String> {
        if count_tokens(&user_input) > token_limits.max_user_tokens {
            return Err("User input exceeds token limit.".to_string());
        }

        let user_log_id = {
            let db_lock = self.db.lock().await;
            db_lock.append_log_return_id("user", &user_input)?
        };

        let global_mem;
        let context_mem;
        let all_chunks;
        {
            let db_lock = self.db.lock().await;
            global_mem = db_lock.get_global_memory()?;
            context_mem = db_lock.get_context()?;
            all_chunks = db_lock.get_all_rag_chunks()?;
        }

        let query_emb =
            call_embedding_api(embed_api.0, embed_api.1, embed_api.2, user_input.clone()).await?;

        let rag_text =
            Self::retrieve_rag_relevant(query_emb, all_chunks, token_limits.max_rag_tokens)?;

        let (summary, history) = Self::parse_context(&context_mem);

        let system_prompt_with_time = format!(
            "{} Current time: {} {}",
            SYSTEM_PROMPT_START,
            Local::now().format("%Y-%m-%dT%H:%M:%S%:z"),
            SYSTEM_PROMPT_END
        );

        let mut total_tokens = count_tokens(&system_prompt_with_time)
            + count_tokens(&global_mem)
            + count_tokens(&rag_text)
            + count_tokens(&summary)
            + count_tokens(&user_input);

        for (_, content) in &history {
            total_tokens += count_tokens(content) + 5;
        }

        if total_tokens <= token_limits.max_total_tokens {
            let ai_content = chat_completion(
                main_api.0,
                main_api.1,
                main_api.2,
                &system_prompt_with_time,
                &global_mem,
                &rag_text,
                &summary,
                history,
                &user_input,
            )
            .await?;

            let ai_res = Self::parse_main_response(ai_content)?;
            let mut ai_log_id = 0; // 0 means no reply
            {
                let db_lock = self.db.lock().await;

                if !ai_res.reply.is_empty() {
                    ai_log_id = db_lock.append_log_return_id("assistant", &ai_res.reply)?;
                    let new_ctx = Self::append_to_context(&context_mem, &user_input, &ai_res.reply);
                    db_lock.set_context(&new_ctx)?;
                }
                // db_lock.set_global_memory(&global_mem)?;
                // db_lock.replace_rag_chunks(&all_chunks)?;
            }
            return Ok(MainAiResponseWithId {
                reply: ai_res.reply,
                proactive_time: ai_res.proactive_time,
                proactive_message: ai_res.proactive_message,
                user_log_id: user_log_id,
                ai_log_id: ai_log_id,
            });
        }

        let (summary, history) = Self::parse_context(&context_mem);
        let total_tokens = count_tokens(&context_mem);
        let max_retain_tokens = total_tokens / 4; // 25% tokens for recent history, 75% for compression

        let mut retained_history = Vec::new();
        let mut retained_tokens = 0;

        for (role, content) in history.iter().rev() {
            let prefix = if role == "user" {
                "User: "
            } else {
                "Assistant: "
            };
            let msg_tokens = count_tokens(&format!("{}{}", prefix, content));
            if retained_tokens + msg_tokens <= max_retain_tokens {
                retained_tokens += msg_tokens;
                retained_history.push((role.clone(), content.clone()));
            } else {
                break;
            }
        }
        retained_history.reverse();

        let compress_len = history.len() - retained_history.len();
        let mut to_compress_ctx = format!("[Summary]\n{}\n", summary);
        if compress_len > 0 {
            to_compress_ctx.push_str("[History]\n");
            for (role, content) in &history[..compress_len] {
                let prefix = if role == "user" {
                    "User: "
                } else {
                    "Assistant: "
                };
                to_compress_ctx.push_str(&format!("{}{}\n", prefix, content));
            }
        }

        let compress_future = compress_context(to_compress_ctx, shadow_api);
        let rag_future = self.generate_new_rag(
            context_mem.clone(),
            shadow_api,
            embed_api,
            token_limits.max_rag_append_tokens,
        );

        let (comp_res, new_rag_data) = tokio::join!(compress_future, rag_future);

        let comp_res = comp_res?;

        let mut final_context = format!("[Summary]\n{}\n", comp_res.temporary);
        if !retained_history.is_empty() {
            final_context.push_str("[History]\n");
            for (role, content) in retained_history {
                final_context.push_str(&format!(
                    "{}: {}\n",
                    if role == "user" { "User" } else { "Assistant" },
                    content
                ));
            }
        }

        let main_ai_future = chat_completion(
            main_api.0,
            main_api.1,
            main_api.2,
            &system_prompt_with_time,
            &global_mem,
            &rag_text,
            &final_context,
            vec![],
            &user_input,
        );

        let update_global_future =
            merge_global_memory(comp_res.permanent.clone(), global_mem.clone(), shadow_api);

        let (ai_content, new_global_mem) = tokio::join!(main_ai_future, update_global_future);
        let ai_res = Self::parse_main_response(ai_content?)?;
        let ai_log_id;
        {
            let db_lock = self.db.lock().await;
            ai_log_id = db_lock.append_log_return_id("assistant", &ai_res.reply)?;

            let new_ctx = format!(
                "[Summary]\n{}\n[History]\nUser: {}\nAssistant: {}",
                final_context, user_input, ai_res.reply
            );
            db_lock.set_context(&new_ctx)?;

            db_lock.set_global_memory(&new_global_mem?)?;
            db_lock.append_rag_chunks(&new_rag_data?)?;
        }

        Ok(MainAiResponseWithId {
            reply: ai_res.reply,
            proactive_time: ai_res.proactive_time,
            proactive_message: ai_res.proactive_message,
            user_log_id: user_log_id,
            ai_log_id: ai_log_id,
        })
    }

    pub async fn trigger_proactive_message(
        &self,
        message: String,
        scheduled_time: Option<String>,
    ) -> Result<(), String> {
        let db_lock = self.db.lock().await;
        let timestamp = scheduled_time.unwrap_or_else(|| chrono::Local::now().to_rfc3339());
        db_lock.append_log_with_time("assistant", &message, &timestamp)?;

        let mut ctx = db_lock.get_context()?;
        if ctx.contains("[History]") {
            ctx.push_str(&format!("\nAssistant: {}", message));
        } else {
            ctx.push_str(&format!("\n[History]\nAssistant: {}", message));
        }
        db_lock.set_context(&ctx)?;
        Ok(())
    }

    fn retrieve_rag_relevant(
        query_emb: Vec<f32>,
        all_chunks: Vec<(String, Vec<f32>, String)>,
        max_rag_tokens: usize,
    ) -> Result<String, String> {
        if all_chunks.is_empty() {
            return Ok(String::new());
        }
        let mut scored: Vec<(f32, String)> = all_chunks
            .into_iter()
            .map(|(text, emb, _)| (cosine_similarity(&query_emb, &emb), text))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        let mut texts = Vec::new();
        let mut tokens_used = 0;
        for (score, text) in scored {
            let t = count_tokens(&text);
            if (tokens_used + t > max_rag_tokens) || score <= 0.2 {
                break;
            }
            dbg!(&score, &text);
            texts.push(text);
            tokens_used += t;
        }
        Ok(texts.join("\n\n"))
    }

    /// expect (text, embeddings, timestamp)
    async fn generate_new_rag(
        &self,
        context: String,
        api: (&str, &str, &str),
        embed_api: (&str, &str, &str),
        max_rag_append_tokens: usize,
    ) -> Result<Vec<(String, Vec<f32>, String)>, String> {
        let all_chunks = {
            let db_lock = self.db.lock().await;
            db_lock.get_all_rag_chunks()?
        };
        
        let mut last_chunks_text = Vec::new();
        let mut tokens_used = 0;
        for (text, _, _) in all_chunks.iter().rev() {
            let t = count_tokens(text);
            if tokens_used + t > max_rag_append_tokens {
                break;
            }
            last_chunks_text.push(text.clone());
            tokens_used += t;
        }
        last_chunks_text.reverse();

        let existing_rag_prompt = if last_chunks_text.is_empty() {
            String::from("No existing memory.")
        } else {
            last_chunks_text.join("\n---\n")
        };

        let current_time = Local::now().format("%Y-%m-%d").to_string();

        let prompt = format!(
            r#"Extract NEW facts/preferences from the new conversation missing in the existing memory.
Current date: [{}]

Existing memory:
{}

New conversation:
{}

Rules:
1. Group related details into a single paragraph. NEVER split related context.
2. Every chunk MUST start with `[{}]`.
3. Output a JSON object with a single key "facts" containing an array of strings. Example: {{"facts": ["[{}] Fact 1", "[{}] Fact 2"]}}
4. If nothing new, output exactly: {{"facts": []}}"#,
            current_time, existing_rag_prompt, context, current_time, current_time, current_time
        );

        let res = call_shadow_ai(api.0, api.1, api.2, prompt, true).await?;

        #[derive(serde::Deserialize)]
        struct RagFacts {
            facts: Vec<String>,
        }

        let parsed_facts: RagFacts = serde_json::from_str(&res)
            .map_err(|e| format!("RAG JSON Parse Error: {}. Raw: {}", e, res))?;

        if parsed_facts.facts.is_empty() {
            return Ok(Vec::new());
        }

        let mut new_chunks = Vec::new();

        for text in parsed_facts.facts {
            let clean_text = text.trim().to_string();
            if clean_text.is_empty() {
                continue;
            }

            if let Ok(emb) =
                call_embedding_api(embed_api.0, embed_api.1, embed_api.2, clean_text.clone()).await
            {
                let ts = Local::now().to_rfc3339();
                new_chunks.push((clean_text, emb, ts));
            }
        }

        Ok(new_chunks)
    }

    fn extract_between(s: &str, start: &str, end: &str) -> Option<String> {
        let start_idx = s.find(start)? + start.len();
        let end_idx = s[start_idx..].find(end)?;
        Some(s[start_idx..start_idx + end_idx].trim().to_string())
    }

    fn parse_main_response(content: String) -> Result<MainAiResponse, String> {
        let marker_start = content.find("[PROACTIVE_TIME:");

        let (reply_part, command_part) = match marker_start {
            Some(idx) => {
                let reply = content[..idx].trim_end().to_string();
                let command = &content[idx..];
                (reply, command)
            }
            None => {
                return Ok(MainAiResponse {
                    reply: content.trim().to_string(),
                    proactive_time: None,
                    proactive_message: None,
                });
            }
        };

        let time = Self::extract_between(command_part, "[PROACTIVE_TIME:", "]").unwrap_or_default();
        let msg = Self::extract_between(command_part, "[PROACTIVE_MSG:", "]").unwrap_or_default();

        if !time.is_empty()
            && chrono::DateTime::parse_from_rfc3339(&time).is_ok()
            && !msg.is_empty()
        {
            return Ok(MainAiResponse {
                reply: reply_part,
                proactive_time: Some(time.to_string()),
                proactive_message: Some(msg.to_string()),
            });
        }

        Ok(MainAiResponse {
            reply: reply_part,
            proactive_time: None,
            proactive_message: None,
        })
    }
    pub async fn get_chat_logs(
        &self,
        before_id: Option<i64>,
        limit: u32,
    ) -> Result<ChatLogsResponse, String> {
        let db_lock = self.db.lock().await;

        // check has_more by fetching one extra record beyond the requested limit
        let actual_limit = limit + 1;

        let logs = if let Some(id) = before_id {
            db_lock.get_logs_before(id, actual_limit)?
        } else {
            db_lock.get_latest_logs(actual_limit)?
        };

        let has_more = logs.len() as u32 > limit;

        let messages: Vec<ChatMessage> = logs
            .into_iter()
            .take(limit as usize)
            .rev()
            .map(
                |(id, role, content, timestamp, grammar_json, parsed_json)| ChatMessage {
                    id,
                    role,
                    content,
                    timestamp,
                    grammar_corrections: grammar_json,
                    parsed_content: parsed_json,
                },
            )
            .collect();
        Ok(ChatLogsResponse { messages, has_more })
    }
}
