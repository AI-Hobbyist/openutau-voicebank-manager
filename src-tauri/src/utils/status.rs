use std::path::Path;
use serde::Serialize;
use std::fs;

#[derive(Serialize)]
pub struct VoicebankStatus {
    pub id: String,
    pub installed: bool,
    pub install_subdir: Option<String>,
    pub local_sha256: Option<String>,
    pub needs_update: bool,
}

#[tauri::command]
pub async fn check_voicebank_status(
    voicebank_path: String,
    voicebanks: Vec<serde_json::Value>,
) -> Result<Vec<VoicebankStatus>, String> {
    let mut statuses = Vec::new();
    let base_path = Path::new(&voicebank_path);

    if !base_path.exists() {
        for vb in voicebanks {
            let id = vb["id"].as_str().unwrap_or("").to_string();
            let install_subdir = vb["install_subdir"].as_str().map(|s| s.to_string());
            statuses.push(VoicebankStatus {
                id,
                installed: false,
                install_subdir,
                local_sha256: None,
                needs_update: false,
            });
        }
        return Ok(statuses);
    }

    for vb in voicebanks {
        let id = vb["id"].as_str().unwrap_or("").to_string();
        let install_subdir = vb["install_subdir"].as_str().map(|s| s.to_string());
        let remote_sha256 = vb["sha256"].as_str().map(|s| s.to_string());
        
        let mut installed = false;
        let mut local_sha256 = None;
        let mut needs_update = false;

        if let Some(ref subdir) = install_subdir {
            let target_path = base_path.join(subdir);
            // 检查目录是否存在且不为空
            if target_path.exists() && target_path.is_dir() {
                if let Ok(entries) = fs::read_dir(&target_path) {
                    let mut has_files = false;
                    for entry in entries.flatten() {
                        if let Ok(name) = entry.file_name().into_string() {
                            if name == "sha256" {
                                if let Ok(content) = fs::read_to_string(entry.path()) {
                                    local_sha256 = Some(content.trim().to_string());
                                }
                            } else {
                                has_files = true;
                            }
                        }
                    }
                    if has_files {
                        installed = true;
                    }
                }

                // 如果有远程 sha256 且本地已经安装，或者本地已经存在 sha256 文件
                if let Some(r_sha) = &remote_sha256 {
                    // 如果本地没有 sha256 文件但已安装，或者本地内容不一致
                    if let Some(l_sha) = &local_sha256 {
                        if l_sha != r_sha {
                            needs_update = true;
                        }
                    } else if installed {
                        // 已安装但没有 sha256 文件，也标记为待更新/需要同步元数据
                        needs_update = true;
                    }
                }
            }
        }

        statuses.push(VoicebankStatus {
            id,
            installed,
            install_subdir,
            local_sha256,
            needs_update,
        });
    }

    Ok(statuses)
}
