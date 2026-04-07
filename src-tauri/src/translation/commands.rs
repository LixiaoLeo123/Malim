
use tauri::State;
use crate::AppState;


#[tauri::command]
pub fn translate(state: State<'_, AppState>, text: &str) -> Result<String, String>{
    state.translator
        .as_ref()
        .map_or(Err("Translator not initialized".into()), |t| t.lock().unwrap().translate(text).map_err(|e| e.to_string()))
}