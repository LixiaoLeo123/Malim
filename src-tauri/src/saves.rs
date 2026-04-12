use tauri::Manager;
use std::fs::File;
use std::io::{copy, Cursor};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;
use zip::read::ZipArchive;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct BackupItem {
    name: String,
    description: String,
    checked: bool,
}

fn get_backup_items() -> Vec<BackupItem> {
    vec![
        BackupItem { name: "data.json".to_string(), description: "User settings & Library".to_string(), checked: true },
        BackupItem { name: "chat.db".to_string(), description: "Chat history".to_string(), checked: true },
        BackupItem { name: "memory.db".to_string(), description: "Vocabulary memory".to_string(), checked: true },
    ]
}

#[tauri::command]
pub fn get_backup_definitions() -> Vec<BackupItem> {
    get_backup_items()
}


#[tauri::command]
pub fn create_export_temp_file(app: tauri::AppHandle, selected_names: Vec<String>) -> Result<Vec<u8>, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(buffer);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for name in selected_names {
        let file_path = data_dir.join(&name);
        if file_path.exists() {
            let mut f = File::open(&file_path).map_err(|e| format!("Failed to open {}: {}", name, e))?;
            zip.start_file(&name, options).map_err(|e| e.to_string())?;
            copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        }
    }

    let buffer = zip.finish().map_err(|e| e.to_string())?;

    Ok(buffer.into_inner())
}

#[tauri::command]
pub fn check_import_file(archive_data: Vec<u8>) -> Result<Vec<String>, String> {
    let reader = Cursor::new(archive_data);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Invalid zip file: {}", e))?;
    
    let valid_names: Vec<String> = get_backup_items().iter().map(|i| i.name.clone()).collect();
    let mut found = Vec::new();

    for i in 0..archive.len() {
        if let Ok(file) = archive.by_index(i) {
            let name = file.name().to_string();
            if valid_names.contains(&name) {
                found.push(name);
            }
        }
    }
    Ok(found)
}

#[tauri::command]
pub fn execute_import(app: tauri::AppHandle, archive_data: Vec<u8>, selected_names: Vec<String>) -> Result<String, String> {
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let reader = Cursor::new(archive_data);
    let mut archive = ZipArchive::new(reader).map_err(|e| format!("Invalid zip file: {}", e))?;

    for name in selected_names {
        let mut file_in_zip = archive.by_name(&name).map_err(|e| format!("File {} not found: {}", name, e))?;
        let out_path = data_dir.join(&name);
        
        let mut outfile = File::create(&out_path).map_err(|e| format!("Failed to create {}: {}", name, e))?;
        copy(&mut file_in_zip, &mut outfile).map_err(|e| e.to_string())?;
    }

    Ok("Import successful. Restart app to apply.".to_string())
}
