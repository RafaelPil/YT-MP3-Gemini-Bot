use teloxide::prelude::*;
use teloxide::types::InputFile;
use reqwest::Client;
use tempfile::tempdir;
use serde_json::Value;
use std::error::Error;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use md5::compute as md5_compute;

pub async fn convert_with_youtubemp3(
    bot: &Bot,
    msg: &Message,
    url: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let chat_id = msg.chat.id;
    bot.send_message(chat_id, "⏳ Starting YouTube to MP3 conversion...").await?;

    let video_id = extract_video_id(url).ok_or("Invalid YouTube URL")?;
    log::debug!("Extracted video ID: {}", video_id);

    let api_key = std::env::var("RAPIDAPI_KEY")?;
    let rapidapi_user = std::env::var("RAPIDAPI_USER")?;
    
    // Generate MD5 of username for whitelisting
    let x_run_header = format!("{:x}", md5_compute(rapidapi_user.as_bytes()));

    let client = Client::new();
    let api_url = format!("https://youtube-mp36.p.rapidapi.com/dl?id={}", video_id);

    let mut attempts = 0;
    let max_attempts = 30;

    loop {
        attempts += 1;
        if attempts > max_attempts {
            return Err("Conversion timed out".into());
        }

        let response = client.get(&api_url)
            .header("x-rapidapi-key", api_key.clone())
            .header("x-rapidapi-host", "youtube-mp36.p.rapidapi.com")
            .header("User-Agent", format!("Mozilla/5.0 {}", rapidapi_user))
            .header("X-RUN", x_run_header.clone())
            .send()
            .await?;

        let response_json: Value = response.json().await?;
        log::debug!("API response: {}", response_json);

        match response_json["status"].as_str() {
            Some("ok") => {
                if let Some(mp3_url) = response_json["link"].as_str() {
                    bot.send_message(chat_id, "⬇️ Downloading MP3 file...").await?;
                    return download_and_send(bot, chat_id, mp3_url, &rapidapi_user, &x_run_header).await;
                }
                return Err("API returned success but no download link".into());
            },
            Some("processing") => {
                if attempts % 5 == 0 {
                    bot.send_message(chat_id, format!("🔄 Still processing (attempt {}/{})", attempts, max_attempts)).await?;
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            },
            Some("fail") => {
                let error_msg = response_json["msg"].as_str().unwrap_or("Conversion failed");
                return Err(error_msg.into());
            },
            _ => return Err("Invalid API response".into()),
        }
    }
}

async fn download_and_send(
    bot: &Bot,
    chat_id: ChatId,
    mp3_url: &str,
    rapidapi_user: &str,
    x_run_header: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let temp_dir = tempdir()?;
    let output_path = temp_dir.path().join("converted.mp3");

    let mut response = client.get(mp3_url)
        .header("User-Agent", format!("Mozilla/5.0 {}", rapidapi_user))
        .header("X-RUN", x_run_header)
        .send()
        .await?;

    let mut file = File::create(&output_path).await?;
    let mut downloaded: u64 = 0;
    
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        if downloaded % (1024 * 1024) == 0 {
            bot.send_chat_action(chat_id, teloxide::types::ChatAction::UploadDocument).await?;
        }
    }

    let metadata = tokio::fs::metadata(&output_path).await?;
    if metadata.len() == 0 {
        return Err("Downloaded file is empty".into());
    }

    bot.send_audio(chat_id, InputFile::file(&output_path)).await?;
    bot.send_message(chat_id, "✅ Conversion complete!").await?;

    temp_dir.close()?;
    Ok(())
}

fn extract_video_id(url: &str) -> Option<String> {
    // Standard YouTube URL formats
    if url.contains("youtube.com/watch?v=") {
        url.split("v=").nth(1).and_then(|s| s.split('&').next()).map(|s| s.to_string())
    } else if url.contains("youtu.be/") {
        url.split("youtu.be/").nth(1).and_then(|s| s.split('?').next()).map(|s| s.to_string())
    }
    // Your specific googleusercontent.com URLs (if they are still relevant/used by your API)
    else if url.contains("youtu.be/") { // Assuming this is an example pattern from your API
        url.split("youtu.be/").nth(1).and_then(|s| s.split('&').next()).map(|s| s.to_string())
    } else if url.contains("youtube.com/watch?v=") { // Assuming this is another example pattern from your API
        url.split("youtube.com/watch?v=").nth(1).and_then(|s| s.split('&').next()).map(|s| s.to_string())
    }
    else {
        None
    }
}