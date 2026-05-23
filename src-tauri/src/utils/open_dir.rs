use std::process::Command;
use std::path::PathBuf;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub fn open_dir(path: String) -> Result<(), String> {
    // 1. 处理路径：如果是相对路径，则直接相对于程序当前运行目录（CWD）
    let mut final_path = PathBuf::from(&path);
    
    if final_path.is_relative() {
        if let Ok(current_dir) = std::env::current_dir() {
            final_path = current_dir.join(&path);
        }
    }

    // 2. 检查路径是否存在
    if !final_path.exists() {
        return Err(format!("路径不存在: {}", final_path.display()));
    }

    let path_str = final_path.to_string_lossy().to_string();

    // 3. 执行打开指令
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("explorer");
        cmd.arg(&path_str);
        
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path_str)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
