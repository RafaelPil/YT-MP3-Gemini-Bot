use teloxide::prelude::*;

mod converter;
mod gemini;

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        if let Some(text) = msg.text() {
            if text.contains("youtube.com") || text.contains("youtu.be") {
                match converter::convert_with_youtubemp3(&bot, &msg, text).await {
                    Ok(_) => (),
                    Err(e) => {
                        log::error!("Conversion error: {}", e);
                        bot.send_message(msg.chat.id, format!("Error: {}", e)).await?;
                    }
                }
            } else {
                match gemini::ask_gemini_flash(text).await {
                    Ok(response) => {
                        bot.send_message(msg.chat.id, response).await?;
                    },
                    Err(e) => {
                        log::error!("Gemini error: {}", e);
                        bot.send_message(msg.chat.id, "Sorry, I couldn't process your request.").await?;
                    }
                }
            }
        }
        Ok(())
    })
    .await;
}