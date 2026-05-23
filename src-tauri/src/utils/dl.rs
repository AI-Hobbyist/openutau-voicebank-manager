use std::process::{Command, Stdio, Child};
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::fs;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tauri::{AppHandle, Emitter, Manager, State};
use super::unzip::unzip_file;

pub struct DownloadManager {
    pub processes: Arc<Mutex<HashMap<String, Child>>>,
}

#[tauri::command]
pub fn download_voicebank(
    app: AppHandle,
    manager: State<'_, DownloadManager>,
    aria2_path: String,
    seven_zip_path: String,
    url: String,
    save_path: String,
    threads: u32,
    connections: u32,
    install_id: String,
    install_subdir: Option<String>,
    sha256: Option<String>,
) -> Result<(), String> {
    if !Path::new(&aria2_path).exists() {
        return Err(format!("找不到 aria2c 可执行文件: {}", aria2_path));
    }

    let processes = manager.processes.clone();
    let app_handle = app.clone();
    let install_id_clone = install_id.clone();
    let save_path_clone = save_path.clone();
    let seven_zip_path_clone = seven_zip_path.clone();
    let sha256_clone = sha256.clone();
    let install_subdir_clone = install_subdir.clone();

    std::thread::spawn(move || {
        let zip_filename = format!("{}.zip", install_id_clone);
        let zip_path = Path::new(&save_path_clone).join(&zip_filename);
        
        // 优先使用 install_subdir 作为安装目录，否则退回到使用 install_id
        let target_subdir = install_subdir_clone.unwrap_or_else(|| install_id_clone.clone());
        let dest_dir = Path::new(&save_path_clone).join(&target_subdir);

        let child_res = Command::new(&aria2_path)
            .arg(&url)
            .arg("-d")
            .arg(&save_path_clone)
            .arg("-o")
            .arg(&zip_filename)
            .arg("-x")
            .arg(threads.to_string())
            .arg("-s")
            .arg(connections.to_string())
            .arg("-c")
            .arg("--summary-interval=1")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        let mut child = match child_res {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit("download-error", format!("启动失败: {}", e));
                return;
            }
        };

        {
            let mut procs = processes.lock().unwrap();
            procs.insert(install_id_clone.clone(), child);
        }

        // 重新获取进程句柄以读取输出
        let mut procs = processes.lock().unwrap();
        if let Some(child_ref) = procs.get_mut(&install_id_clone) {
            let stdout = child_ref.stdout.take().unwrap();
            let reader = BufReader::new(stdout);
            
            // 释放锁以便其他操作（如下拉）
            drop(procs);

            for line in reader.lines() {
                if let Ok(l) = line {
                    // 解析进度
                    if let Some(start) = l.find('(') {
                        if let Some(end) = l.find("%)") {
                            let percent_str = &l[start + 1..end];
                            if let Ok(percent) = percent_str.parse::<f32>() {
                                let _ = app_handle.emit("download-progress", percent);
                            }
                        }
                    }
                    // 解析速度 (aria2c 典型的输出格式包含 DL:XXMiB/s)
                    if let Some(dl_idx) = l.find("DL:") {
                        let speed_part = &l[dl_idx + 3..];
                        if let Some(space_idx) = speed_part.find(' ') {
                            let speed = &speed_part[..space_idx];
                            let _ = app_handle.emit("download-speed", speed.to_string());
                        }
                    }
                    // 解析剩余时间 (aria2c 典型的输出格式包含 ETA:XXhXXmXXs)
                    if let Some(eta_idx) = l.find("ETA:") {
                        let eta_part = &l[eta_idx + 4..];
                        if let Some(space_idx) = eta_part.find(' ') {
                            let eta = &eta_part[..space_idx];
                            let _ = app_handle.emit("download-eta", eta.to_string());
                        }
                    }
                }
            }
        } else {
            return;
        }

        // 等待进程结束并清理
        let mut procs = processes.lock().unwrap();
        if let Some(mut child) = procs.remove(&install_id_clone) {
            let status = child.wait().expect("aria2c failed to finish");
            drop(procs);

            if status.success() {
                let _ = app_handle.emit("download-progress", 99.0);
                
                // 等待一小会儿确保文件句柄完全释放（特别是在 Windows 上）
                std::thread::sleep(std::time::Duration::from_millis(500));

                match unzip_file(&seven_zip_path_clone, &zip_path, &dest_dir) {
                    Ok(_) => {
                        let _ = fs::remove_file(&zip_path);
                        
                        // 写入 SHA256 文件
                        if let Some(sha) = sha256_clone {
                            let sha_file_path = dest_dir.join("sha256");
                            let _ = fs::write(sha_file_path, sha);
                        }

                        let _ = app_handle.emit("download-finished", true);
                    },
                    Err(e) => {
                        // 如果 7-Zip 都无法解压，说明文件极大可能已损坏或非压缩格式，直接清理掉
                        if zip_path.exists() {
                            let _ = fs::remove_file(&zip_path);
                        }
                        let _ = app_handle.emit("download-error", format!("解压失败（损坏的文件已自动清理）: {}", e));
                    }
                }
            } else {
                // 如果 zip 还在，说明是中途停止或失败，清理临时文件
                if zip_path.exists() {
                    let _ = fs::remove_file(&zip_path);
                }
                let _ = app_handle.emit("download-error", "下载已取消或出错");
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub fn cancel_download(
    manager: State<'_, DownloadManager>,
    install_id: String,
) -> Result<(), String> {
    let mut procs = manager.processes.lock().unwrap();
    if let Some(mut child) = procs.remove(&install_id) {
        let _ = child.kill();
    }
    Ok(())
}
