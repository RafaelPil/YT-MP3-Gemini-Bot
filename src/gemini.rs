use serde_json::Value;
use std::error::Error;

pub async fn ask_gemini_flash(query: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key={}", 
        api_key
    );

    let body = serde_json::json!({
        "contents": [{
            "parts": [{
                "text": query
            }]
        }],
        "generationConfig": {
            "temperature": 0.9,
            "topP": 1.0,
            "maxOutputTokens": 2048
        }
    });

    let client = reqwest::Client::new();
    let res = client.post(&url)
        .json(&body)
        .send()
        .await?;

    let response_text = res.text().await?;
    log::debug!("Raw Gemini response: {}", response_text);

    let response_json: Value = serde_json::from_str(&response_text)?;

    if let Some(error) = response_json.get("error") {
        return Err(format!("Gemini API error: {}", error).into());
    }

    response_json["candidates"][0]["content"]["parts"][0]["text"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "Failed to parse Gemini response".into())
}