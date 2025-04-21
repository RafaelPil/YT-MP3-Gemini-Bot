use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

mod converter;
mod gemini;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "🎵 Download YouTube video as MP3", parse_with = "split")]
    Download,
    #[command(description = "🤖 Ask Gemini AI a question", parse_with = "split")]
    Ai,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();

    // Register commands with Telegram
    bot.set_my_commands(Command::bot_commands())
        .await
        .expect("Failed to set commands");

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        if let Some(text) = msg.text() {
            // Try to parse as command first
            if let Ok(command) = Command::parse(text, "") {
                match command {
                    Command::Download => {
                        bot.send_message(msg.chat.id, "Please send me the YouTube URL you want to convert to MP3").await?;
                    }
                    Command::Ai => {
                        bot.send_message(msg.chat.id, "What would you like to ask AI?").await?;
                    }
                }
                return Ok(());
            }

            // Handle responses to command prompts
            if let Some(reply) = msg.reply_to_message() {
                if let Some(reply_text) = reply.text() {
                    if reply_text.contains("YouTube URL") {
                        if text.contains("youtube.com") || text.contains("youtu.be") {
                            if let Err(e) = converter::convert_with_youtubemp3(&bot, &msg, text).await {
                                log::error!("Conversion error: {}", e);
                                bot.send_message(msg.chat.id, format!("Error: {}", e)).await?;
                            }
                        } else {
                            bot.send_message(msg.chat.id, "That doesn't look like a YouTube URL. Please send a valid YouTube URL.").await?;
                        }
                        return Ok(());
                    } else if reply_text.contains("ask Gemini AI") {
                        match gemini::ask_gemini_flash(text).await {
                            Ok(response) => {
                                let _ = bot.send_message(msg.chat.id, response).await?;
                            },
                            Err(e) => {
                                log::error!("Gemini error: {}", e);
                                let _ = bot.send_message(msg.chat.id, "Sorry, I couldn't process your request.").await?;
                            }
                        }
                        return Ok(());
                    }
                }
            }

            // Original behavior for non-command messages
            if text.contains("youtube.com") || text.contains("youtu.be") {
                if let Err(e) = converter::convert_with_youtubemp3(&bot, &msg, text).await {
                    log::error!("Conversion error: {}", e);
                    bot.send_message(msg.chat.id, format!("Error: {}", e)).await?;
                }
            } else {
                match gemini::ask_gemini_flash(text).await {
                    Ok(response) => bot.send_message(msg.chat.id, response).await.map(|_| ())?,
                    Err(e) => {
                        log::error!("Gemini error: {}", e);
                        let _ = bot.send_message(msg.chat.id, "Sorry, I couldn't process your request.").await?;
                    }
                }
            }
        }
        Ok(())
    })
    .await;
}