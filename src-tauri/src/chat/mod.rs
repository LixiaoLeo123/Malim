pub mod ai;
pub mod commands;
pub mod db;
pub mod token;
pub mod vector;

use ai::{
    call_embedding_api, call_shadow_ai, chat_completion, compress_context, merge_global_memory,
    GrammarCorrection, MainAiResponse, MainAiResponseWithId, SYSTEM_PROMPT,
};
use chrono::Local;
use db::DbState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use token::count_tokens;
use tokio::sync::Mutex;
use vector::cosine_similarity;

const MAX_TOTAL_TOKENS: usize = 400;
const MAX_RAG_TOKENS: usize = 100;
const MAX_RAG_APPEND_TOKENS: usize = 100; // token used to append new content with previous rag
const MAX_USER_TOKENS: usize = 100;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub grammar_corrections: Option<String>, // JSON string of Vec<GrammarCorrection>
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
    ) -> Result<MainAiResponseWithId, String> {
        if count_tokens(&user_input) > MAX_USER_TOKENS {
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

        let rag_text = Self::retrieve_rag_relevant(query_emb, all_chunks)?;

        let (summary, history) = Self::parse_context(&context_mem);

        let mut total_tokens = count_tokens(SYSTEM_PROMPT)
            + count_tokens(&global_mem)
            + count_tokens(&rag_text)
            + count_tokens(&summary)
            + count_tokens(&user_input);

        for (_, content) in &history {
            total_tokens += count_tokens(content) + 5;
        }

        if total_tokens <= MAX_TOTAL_TOKENS {
            let ai_content = chat_completion(
                main_api.0,
                main_api.1,
                main_api.2,
                SYSTEM_PROMPT,
                &global_mem,
                &rag_text,
                &summary,
                history,
                &user_input,
            )
            .await?;

            let ai_res = Self::parse_main_response(ai_content)?;

            {
                let db_lock = self.db.lock().await;
                if !ai_res.reply.is_empty() {
                    db_lock.append_log("assistant", &ai_res.reply)?;
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
            });
        }

        let compress_future = compress_context(context_mem.clone(), shadow_api);
        let rag_future = self.generate_new_rag(context_mem.clone(), shadow_api, embed_api);

        let (comp_res, new_rag_data) = tokio::join!(compress_future, rag_future);
        let comp_data = comp_res?;

        let main_ai_future = chat_completion(
            main_api.0,
            main_api.1,
            main_api.2,
            SYSTEM_PROMPT,
            &global_mem,
            &rag_text,
            &comp_data.temporary,
            vec![],
            &user_input,
        );

        let update_global_future =
            merge_global_memory(comp_data.permanent.clone(), global_mem.clone(), shadow_api);

        let (ai_content, new_global_mem) = tokio::join!(main_ai_future, update_global_future);
        let ai_res = Self::parse_main_response(ai_content?)?;

        {
            let db_lock = self.db.lock().await;
            db_lock.append_log("assistant", &ai_res.reply)?;

            let new_ctx = format!(
                "[Summary]\n{}\n[History]\nUser: {}\nAssistant: {}",
                comp_data.temporary, user_input, ai_res.reply
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
            if tokens_used + t > MAX_RAG_TOKENS {
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
    ) -> Result<Vec<(String, Vec<f32>, String)>, String> {
        let all_chunks = {
            let db_lock = self.db.lock().await;
            db_lock.get_all_rag_chunks()?
        };
        let mut new_chunks = Vec::new();
        let mut last_chunks_text = Vec::new();
        let mut tokens_used = 0;
        for (text, _, _) in all_chunks.iter().rev() {
            let t = count_tokens(text);
            if tokens_used + t > MAX_RAG_APPEND_TOKENS {
                break;
            }
            last_chunks_text.push(text.clone());
            tokens_used += t;
        }
        last_chunks_text.reverse();

        let existing_rag_prompt = if last_chunks_text.is_empty() {
            String::from("No existing RAG chunks.")
        } else {
            last_chunks_text.join("\n---\n")
        };

        let current_time = Local::now().format("%Y-%m-%d").to_string();

        let prompt = format!(
    "Extract NEW facts/preferences from the new conversation missing in the existing memory.\n\
     Current date: [{}]\n\n\
     Existing memory:\n{}\n\n\
     New conversation:\n{}\n\n\
     Rules:\n\
     1. Group related details into a single paragraph. NEVER split related context into multiple chunks.\n\
     2. Every chunk must start with `[{}]`.\n\
     3. Separate chunks with double newlines.\n\
     Output 'NONE' if nothing new.",
    current_time, existing_rag_prompt, context, current_time
        );

        let res = call_shadow_ai(api.0, api.1, api.2, prompt).await?;

        if res.trim().eq_ignore_ascii_case("NONE") || res.trim().is_empty() {
            return Ok(new_chunks);
        }

        let new_texts: Vec<String> = res
            .split("\n\n")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        for text in new_texts {
            if let Ok(emb) =
                call_embedding_api(embed_api.0, embed_api.1, embed_api.2, text.clone()).await
            {
                let ts = Local::now().to_rfc3339();
                new_chunks.push((text, emb, ts));
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
            .map(|(id, role, content, timestamp, grammar_json)| ChatMessage {
                id,
                role,
                content,
                timestamp,
                grammar_corrections: grammar_json,
            })
            .collect();
        Ok(ChatLogsResponse { messages, has_more })
    }
}
