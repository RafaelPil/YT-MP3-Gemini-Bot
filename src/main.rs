use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use futures::FutureExt;
use std::env; // For environment variables

mod converter;
mod usage_manager;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "🎵 Download YouTube video as MP3", parse_with = "split")]
    Download,
    // Admin command for setting premium status
    #[command(description = "👑 (Admin) Grant or revoke premium access. Usage: /setpremium <user_id> <true|false>", parse_with = "split")]
    SetPremium,
    #[command(description = "❓ Get your Telegram User ID")]
    MyId,
    #[command(description = "📊 (Admin) Display current user usage statistics")] // NEW COMMAND
    Usage,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().expect("Failed to read .env file");
    pretty_env_logger::init();
    log::info!("Starting bot...");

    let bot = Bot::from_env();

    // Get the bot owner's chat ID from environment variables
    let bot_owner_chat_id: i64 = env::var("BOT_OWNER_CHAT_ID")
        .expect("BOT_OWNER_CHAT_ID must be set in .env")
        .parse()
        .expect("BOT_OWNER_CHAT_ID must be a valid integer");
    log::info!("Bot owner chat ID: {}", bot_owner_chat_id);

    // Initialize UsageManager
    let usage_manager = Arc::new(usage_manager::UsageManager::new().await.expect("Failed to initialize usage manager"));

    // Spawn a task for periodic saving
    let usage_manager_clone_for_save = Arc::clone(&usage_manager);
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60 * 5)); // Save every 5 minutes
        loop {
            interval.tick().await;
            if let Err(e) = usage_manager_clone_for_save.save_usage().await {
                log::error!("Failed to save usage data: {}", e);
            }
        }
    });

    // Handle shutdown gracefully to save data
    let usage_manager_shutdown_clone = Arc::clone(&usage_manager);

    // Create a signal handler for graceful shutdown
    let shutdown_signal = tokio::signal::ctrl_c().fuse();

    // Spawn the REPL as a background task
    let usage_manager_for_repl = Arc::clone(&usage_manager);
    let bot_for_repl = bot.clone();
    tokio::spawn(async move {
        teloxide::repl(bot_for_repl, move |bot: Bot, msg: Message| {
            let usage_manager = Arc::clone(&usage_manager_for_repl); // Clone for each message handler
            let bot_owner_chat_id_for_closure = bot_owner_chat_id; // Copy owner ID
            async move {
                let chat_id = msg.chat.id;
                if let Some(text) = msg.text() {
                    // Try to parse as command first
                    if let Ok(command) = Command::parse(text, "") {
                        match command {
                            Command::Download => {
                                bot.send_message(chat_id, "Please send me the YouTube URL you want to convert to MP3").await?;
                            }
                            Command::SetPremium => { // Handle SetPremium command
                                if chat_id.0 != bot_owner_chat_id_for_closure {
                                    bot.send_message(chat_id, "🚫 You are not authorized to use this command.").await?;
                                    return Ok(());
                                }

                                let parts: Vec<&str> = text.split_whitespace().collect();
                                if parts.len() != 3 {
                                    bot.send_message(chat_id, "Usage: /setpremium <user_id> <true|false>").await?;
                                    return Ok(());
                                }

                                let target_chat_id_str = parts[1];
                                let status_str = parts[2];

                                let target_chat_id: i64 = match target_chat_id_str.parse() {
                                    Ok(id) => id,
                                    Err(_) => {
                                        bot.send_message(chat_id, "Invalid user_id. Must be a number.").await?;
                                        return Ok(());
                                    }
                                };

                                let status: bool = match status_str.parse() {
                                    Ok(s) => s,
                                    Err(_) => {
                                        bot.send_message(chat_id, "Invalid status. Must be 'true' or 'false'.").await?;
                                        return Ok(());
                                    }
                                };

                                let changed = usage_manager.set_premium_status(target_chat_id, status).await;

                                if changed {
                                    bot.send_message(chat_id, format!("Premium status for user {} set to {}.", target_chat_id, status)).await?;
                                    // Optionally notify the target user
                                    let status_text = if status { "unlimited downloads!" } else { "standard free tier (10 downloads/week)." };
                                    bot.send_message(ChatId(target_chat_id), format!("Your access has been updated! You now have {}.", status_text)).await?;
                                } else {
                                    bot.send_message(chat_id, format!("Premium status for user {} was already {}. No change made.", target_chat_id, status)).await?;
                                }
                            }
                            Command::MyId => {
                                bot.send_message(chat_id, format!("Your Telegram User ID is: `{}`", chat_id.0)).await?;
                            }
                            Command::Usage => { // NEW USAGE COMMAND HANDLER
                                if chat_id.0 != bot_owner_chat_id_for_closure {
                                    bot.send_message(chat_id, "🚫 You are not authorized to use this command.").await?;
                                    return Ok(());
                                }

                                match usage_manager.get_all_usage_summary().await {
                                    Ok(summary) => {
                                        if summary.is_empty() {
                                            bot.send_message(chat_id, "No usage data available yet.").await?;
                                        } else {
                                            let mut response = String::from("📊 **User Usage Summary:**\n\n");
                                            response.push_str("`User ID        | Downloads | Premium`\n");
                                            response.push_str("`------------------------------------`\n");
                                            for (user_id, stats) in summary {
                                                response.push_str(&format!(
                                                    "`{: <15} | {: <9} | {: <7}`\n",
                                                    user_id, stats.downloads_this_week, stats.is_premium
                                                ));
                                            }
                                            bot.send_message(chat_id, response)
                                                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                                                .await?;
                                        }
                                    },
                                    Err(e) => {
                                        log::error!("Failed to get usage summary: {}", e);
                                        bot.send_message(chat_id, format!("Error retrieving usage data: {}", e)).await?;
                                    }
                                }
                            }
                        }
                        return Ok(());
                    }

                    // Handle responses to command prompts
                    if let Some(reply) = msg.reply_to_message() {
                        if let Some(reply_text) = reply.text() {
                            if reply_text.contains("YouTube URL") {
                                let (can_download, current_downloads, limit) = usage_manager.can_download(chat_id.0).await;

                                // Custom message if user is premium and bypasses the limit
                                if usage_manager.get_user_stats(chat_id.0).await.map_or(false, |s| s.is_premium) {
                                    bot.send_message(chat_id, "You have unlimited downloads. Processing your request!").await?;
                                } else if !can_download {
                                    bot.send_message(chat_id, format!(
                                        "🚫 You've reached your weekly limit of {} free songs. You've downloaded {} songs this week.\n\n\
                                         Want unlimited downloads? Contact us! ➡️ @Vibedownload", // Updated contact info
                                        limit, current_downloads
                                    )).await?;
                                    return Ok(());
                                } else {
                                    bot.send_message(chat_id, format!("You have {} downloads remaining this week.", limit - current_downloads)).await?;
                                }

                                if text.contains("youtube.com/watch?") || text.contains("youtu.be/") { // More robust YouTube URL check
                                    if let Err(e) = converter::convert_with_youtubemp3(&bot, &msg, text).await {
                                        log::error!("Conversion error: {}", e);
                                        bot.send_message(chat_id, format!("Error: {}", e)).await?;
                                    } else {
                                        usage_manager.record_download(chat_id.0).await;
                                    }
                                } else {
                                    bot.send_message(chat_id, "That doesn't look like a YouTube URL. Please send a valid YouTube URL.").await?;
                                }
                                return Ok(());
                            }
                        }
                    }

                    // Original behavior for non-command messages (if they are URLs directly)
                    if text.contains("youtube.com/watch?") || text.contains("youtu.be/") { // More robust YouTube URL check
                        let (can_download, current_downloads, limit) = usage_manager.can_download(chat_id.0).await;

                        // Custom message if user is premium and bypasses the limit
                        if usage_manager.get_user_stats(chat_id.0).await.map_or(false, |s| s.is_premium) {
                            bot.send_message(chat_id, "You have unlimited downloads. Processing your request!").await?;
                        } else if !can_download {
                            bot.send_message(chat_id, format!(
                                "🚫 You've reached your weekly limit of {} free songs. You've downloaded {} songs this week.\n\n\
                                 Want unlimited downloads? Contact us! ➡️ @Vibedownload", // Updated contact info
                                limit, current_downloads
                            )).await?;
                            return Ok(());
                        } else {
                            bot.send_message(chat_id, format!("You have {} downloads remaining this week.", limit - current_downloads)).await?;
                        }

                        if let Err(e) = converter::convert_with_youtubemp3(&bot, &msg, text).await {
                            log::error!("Conversion error: {}", e);
                            bot.send_message(chat_id, format!("Error: {}", e)).await?;
                        } else {
                            usage_manager.record_download(chat_id.0).await;
                        }
                    } else {
                        bot.send_message(chat_id, "I can only process YouTube URLs for MP3 conversion. Please send a valid YouTube URL.").await?;
                    }
                }
                Ok(())
            }
        }).await;
    });

    // Wait for the shutdown signal (Ctrl+C)
    shutdown_signal.await.expect("Failed to receive shutdown signal");
    log::info!("Shutdown signal received. Stopping bot...");
    // Optionally, you can abort the REPL task if needed:
    // repl_handle.abort();

    log::info!("Shutting down. Saving usage data...");
    if let Err(e) = usage_manager_shutdown_clone.save_usage().await {
        log::error!("Failed to save usage data during shutdown: {}", e);
    }
    log::info!("Bot stopped.");
}