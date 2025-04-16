# Telegram YouTube to MP3 Bot with Gemini AI

A Rust-based Telegram bot that converts YouTube videos to MP3 and provides AI responses using Google's Gemini.

## Features

- 🎵 Convert YouTube videos to MP3
- 🤖 AI-powered responses using Gemini 2.0 Flash
- ⚡ Fast conversion via RapidAPI
- 🔒 Secure file handling with temporary storage
- 🔄 Real-time status updates

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- Telegram Bot Token from [@BotFather](https://t.me/BotFather)
- [RapidAPI Key](https://rapidapi.com/ytjar/api/youtube-mp36)
- [Google Gemini API Key](https://ai.google.dev/)

## Installation and Running

### 1. Clone the Repository:

```bash
git clone https://github.com/yourusername/telegram-mp3-bot.git
cd telegram-mp3-bot
```

### 2. Configure Environment Variables:

    ```
    TELOXIDE_TOKEN=your_telegram_bot_token
    RAPIDAPI_KEY=your_rapidapi_key
    RAPIDAPI_USER=your_rapidapi_username
    GEMINI_API_KEY=your_gemini_api_key
    ```

### 3. Build and Run:

    ```
    cargo build --release
    cargo run --release
    ```

## Troubleshooting:

- 404 Errors: Verify your RapidAPI credentials
- Conversion Failures: Check your API quota
- Bot Not Responding: Validate all environment variables

## Support:

- For issues, please open an issue.

## Note:

This project is for educational purposes only.
