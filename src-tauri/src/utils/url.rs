use reqwest::Client;
use std::time::Duration;

#[tauri::command]
pub async fn check_url_status(url: String) -> Result<u16, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let response = client.head(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(response.status().as_u16())
}
