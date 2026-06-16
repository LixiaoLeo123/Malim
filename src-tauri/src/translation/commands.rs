use tauri::{AppHandle, Manager};
use tauri_plugin_fs::FsExt;
use crate::translation;

const MODEL_FILENAME: &str = "Hy-MT2-1.8B-Q4_0.gguf";

fn get_app_data(app: &AppHandle) -> std::path::PathBuf {
    app.path().app_data_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

/// Ensure the model file exists in app_data_dir/models/, reading it from the bundled
/// Tauri resources on first run. Uses tauri-plugin-fs which handles the asset://
/// protocol on Android.
///
/// On Android, resource_dir() returns `asset://localhost/` and the model is at
/// `assets/resources/` inside the APK, so the full path is `asset://localhost/resources/<file>`.
/// On desktop, resource_dir() points to the install dir where files from
/// `resources/*` are placed directly, so just `<file>` suffices.
fn ensure_model(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let app_data = get_app_data(app);
    let target_dir = app_data.join("models");
    std::fs::create_dir_all(&target_dir).map_err(|e| format!("mkdir: {}", e))?;
    let target = target_dir.join(MODEL_FILENAME);
    if target.exists() {
        return Ok(target);
    }

    let resource_dir = app.path().resource_dir().map_err(|e| format!("resource_dir(): {}", e))?;

    #[cfg(target_os = "android")]
    let resource_model = resource_dir.join("resources").join(MODEL_FILENAME);
    #[cfg(not(target_os = "android"))]
    let resource_model = resource_dir.join(MODEL_FILENAME);

    let rm = resource_model.display().to_string();
    eprintln!("[translation] reading model from: {}", rm);

    let model_bytes = app.fs().read(&resource_model)
        .map_err(|e| format!("fs().read({}): {}", rm, e))?;

    eprintln!("[translation] read {} bytes, writing to {}", model_bytes.len(), target.display());

    std::fs::write(&target, &model_bytes)
        .map_err(|e| format!("Failed to write model: {}", e))?;

    Ok(target)
}

#[tauri::command]
pub async fn translate(app: AppHandle, text: String) -> Result<String, String> {
    let app_data = get_app_data(&app);
    ensure_model(&app)?;
    let translator = translation::get_translator(&app_data)
        .map_err(|e| format!("Translator init failed: {}", e))?;
    tokio::task::spawn_blocking(move || {
        let t = translator.lock().map_err(|e| format!("Lock: {}", e))?;
        t.translate(&text, "EN", "RU")
            .map_err(|e| format!("Translation: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

#[tauri::command]
pub async fn translate_llm(
    app: AppHandle,
    text: String,
    source_lang: String,
    target_lang: String,
) -> Result<String, String> {
    if text.trim().is_empty() {
        return Err("No text provided".into());
    }
    let app_data = get_app_data(&app);
    ensure_model(&app)?;
    let translator = translation::get_translator(&app_data)
        .map_err(|e| format!("Translator init failed: {}", e))?;
    tokio::task::spawn_blocking(move || {
        let t = translator.lock().map_err(|e| format!("Lock: {}", e))?;
        t.translate(&text, &source_lang, &target_lang)
            .map_err(|e| format!("Translation: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}
