use crate::chat::ChatLogsResponse;
use crate::chat::GrammarCorrection;
use crate::chat::MainAiResponseWithId;
use crate::chat::TokenLimits;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub async fn save_grammar_corrections(
    state: State<'_, AppState>,
    log_id: i64,
    corrections: Vec<GrammarCorrection>,
) -> Result<(), String> {
    let handler = &state.memory_handler;
    handler.save_grammar(log_id, corrections).await
}

#[tauri::command]
pub async fn update_chat_parsed(
    state: State<'_, AppState>,
    log_id: i64,
    parsed_content: String,
) -> Result<(), String> {
    let handler = &state.memory_handler;
    handler.update_parsed_content(log_id, parsed_content).await
}

#[tauri::command]
pub async fn send_message(
    state: State<'_, AppState>,
    user_input: String,
    main_api_key: String,
    main_api_url: String,
    main_model_name: String,
    shadow_api_key: String,
    shadow_api_url: String,
    shadow_model_name: String,
    embed_api_key: String,
    embed_api_url: String,
    embed_model_name: String,
    max_total_tokens: Option<u32>,
    max_rag_tokens: Option<u32>,
    max_rag_append_tokens: Option<u32>,
    max_user_tokens: Option<u32>,
) -> Result<MainAiResponseWithId, String> {
    let _lock = state.chat_lock.lock().await;
    let handler = &state.memory_handler;
    let main_api = (
        main_api_key.as_str(),
        main_api_url.as_str(),
        main_model_name.as_str(),
    );
    let shadow_api = (
        shadow_api_key.as_str(),
        shadow_api_url.as_str(),
        shadow_model_name.as_str(),
    );
    let embed_api = (
        embed_api_key.as_str(),
        embed_api_url.as_str(),
        embed_model_name.as_str(),
    );
    let token_limits = TokenLimits::from_input(
        max_total_tokens,
        max_rag_tokens,
        max_rag_append_tokens,
        max_user_tokens,
    );

    let result = handler
        .handle_message(user_input, main_api, shadow_api, embed_api, token_limits)
        .await;
    dbg!(&result);
    result
}

#[tauri::command]
pub async fn trigger_proactive(
    state: State<'_, AppState>,
    message: String,
    scheduled_time: Option<String>,
) -> Result<(), String> {
    let handler = &state.memory_handler;
    handler
        .trigger_proactive_message(message, scheduled_time)
        .await
}

#[tauri::command]
pub async fn get_chat_logs(
    state: State<'_, AppState>,
    before_id: Option<i64>,
    limit: Option<u32>,
) -> Result<ChatLogsResponse, String> {
    let limit = limit.unwrap_or(20);
    let handler = &state.memory_handler;
    handler.get_chat_logs(before_id, limit).await
}
