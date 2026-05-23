use std::fs;
use std::path::Path;

#[tauri::command]
pub async fn del_dir(path: String) -> Result<(), String> {
    let target_path = Path::new(&path);
    if target_path.exists() {
        if target_path.is_dir() {
            fs::remove_dir_all(target_path).map_err(|e| e.to_string())?;
        } else {
            fs::remove_file(target_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
