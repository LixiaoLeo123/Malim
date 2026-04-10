fn main() {
    let dict_path = "/home/leo/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rsmorphy-dict-ru-0.1.0/data";
    let out_dir = std::path::Path::new("rsmorphy_data");
    let _ = std::fs::create_dir_all(&out_dir);
    if let Ok(dir) = std::fs::read_dir(dict_path) {
        for entry in dir.filter_map(|e| e.ok()) {
            let _ = std::fs::copy(entry.path(), out_dir.join(entry.file_name()));
        }
    }
    tauri_build::build()
}
