use std::path::Path;
use trash;

#[tauri::command]
pub async fn del_dir(path: String) -> Result<(), String> {
    let target_path = Path::new(&path);
    if target_path.exists() {
        trash::delete(target_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
