mod utils;
use utils::open_dir::open_dir;
use utils::dl::{download_voicebank, cancel_download, DownloadManager};
use utils::status::check_voicebank_status;
use utils::del::del_dir;
use utils::url::check_url_status;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .manage(DownloadManager {
        processes: Arc::new(Mutex::new(HashMap::new())),
    })
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![
        open_dir,
        download_voicebank,
        cancel_download,
        check_voicebank_status,
        del_dir,
        check_url_status
    ])
    .setup(|app| {
      if cfg!(debug_assertions) {
        app.handle().plugin(
          tauri_plugin_log::Builder::default()
            .level(log::LevelFilter::Info)
            .build(),
        )?;
      }
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
