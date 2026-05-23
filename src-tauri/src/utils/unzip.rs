use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn unzip_file(seven_zip_path: &str, zip_path: &Path, dest_dir: &Path) -> Result<(), String> {
    if !Path::new(seven_zip_path).exists() {
        return Err(format!("找不到 7-Zip 执行文件: {}", seven_zip_path));
    }

    if !dest_dir.exists() {
        fs::create_dir_all(dest_dir).map_err(|e| format!("无法创建目标目录: {}", e))?;
    }

    // 调用 7za.exe 解压
    // x: 提取文件并保留目录结构
    // -o: 指定输出目录 (注意 -o 和路径之间没有空格)
    // -y: 所有确认问题默认为 "yes"
    let output = Command::new(seven_zip_path)
        .arg("x")
        .arg(zip_path)
        .arg(format!("-o{}", dest_dir.to_string_lossy()))
        .arg("-y")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("启动 7-Zip 失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("7-Zip 解压失败: {}", err))
    }
}
