use std::{collections::HashMap, io::{self, BufReader, BufWriter}};
use serde::{Serialize, Deserialize};
use tokio::sync::Mutex;
use chrono::{Utc, Datelike};
use std::path::{Path, PathBuf};

const USAGE_FILE: &str = "user_usage.json";
const WEEKLY_LIMIT: u32 = 10;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserStats {
    pub downloads_this_week: u32,
    last_reset_week: u32, // ISO week number
    last_reset_year: i32,
    pub is_premium: bool, // <-- NEW FIELD
}

impl Default for UserStats {
    fn default() -> Self {
        let now = Utc::now().date_naive();
        let week = now.iso_week();
        UserStats {
            downloads_this_week: 0,
            last_reset_week: week.week(),
            last_reset_year: week.year(),
            is_premium: false, // <-- Default to not premium
        }
    }
}

pub struct UsageManager {
    usage_data: Mutex<HashMap<i64, UserStats>>, // chat_id -> UserStats
    data_file_path: PathBuf,
}

impl UsageManager {
    pub async fn new() -> io::Result<Self> {
        let data_file_path = Path::new(USAGE_FILE).to_path_buf();
        let usage_data = Self::load_usage(&data_file_path).await?;
        Ok(Self {
            usage_data: Mutex::new(usage_data),
            data_file_path,
        })
    }

    async fn load_usage(path: &PathBuf) -> io::Result<HashMap<i64, UserStats>> {
        if !path.exists() {
            log::info!("Usage data file not found, creating new one.");
            return Ok(HashMap::new());
        }

        let file = tokio::fs::File::open(path).await?.into_std().await;
        let reader = BufReader::new(file);
        // Deserialize with a custom visitor or default for new fields if file schema changes frequently
        let usage: HashMap<i64, UserStats> = serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("Failed to parse usage data: {}", e)))?;

        log::info!("Loaded usage data from {}", path.display());
        Ok(usage)
    }

    pub async fn save_usage(&self) -> io::Result<()> {
        let usage_data = self.usage_data.lock().await;
        let file = tokio::fs::File::create(&self.data_file_path).await?.into_std().await;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*usage_data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to write usage data: {}", e)))?;
        log::info!("Saved usage data to {}", self.data_file_path.display());
        Ok(())
    }

    pub async fn can_download(&self, chat_id: i64) -> (bool, u32, u32) {
        let mut data = self.usage_data.lock().await;
        let now = Utc::now().date_naive();
        let current_week_info = now.iso_week();

        let stats = data.entry(chat_id).or_insert_with(UserStats::default);

        // If the user is premium, they can always download
        if stats.is_premium { // <-- NEW CHECK
            return (true, 0, 0); // Return true, and 0 for current/limit as they are unlimited
        }

        // Check if a new week has started and reset counts
        if stats.last_reset_year != current_week_info.year() || stats.last_reset_week != current_week_info.week() {
            log::info!("Resetting downloads for user {} (old week: {}/{}, new week: {}/{})",
                       chat_id, stats.last_reset_year, stats.last_reset_week, current_week_info.year(), current_week_info.week());
            stats.downloads_this_week = 0;
            stats.last_reset_week = current_week_info.week();
            stats.last_reset_year = current_week_info.year();
        }

        let allowed = stats.downloads_this_week < WEEKLY_LIMIT;
        (allowed, stats.downloads_this_week, WEEKLY_LIMIT)
    }

    pub async fn record_download(&self, chat_id: i64) {
        let mut data = self.usage_data.lock().await;
        if let Some(stats) = data.get_mut(&chat_id) {
            // Only increment if not premium
            if !stats.is_premium { // <-- Only record if not premium
                stats.downloads_this_week += 1;
                log::info!("Recorded download for user {}. Total this week: {}", chat_id, stats.downloads_this_week);
            } else {
                log::info!("User {} is premium, no download count recorded.", chat_id);
            }
        }
    }

    // NEW METHOD to set premium status
    pub async fn set_premium_status(&self, chat_id: i64, status: bool) -> bool {
        let mut data = self.usage_data.lock().await;
        // Ensure the user exists or create a default entry for them
        let stats = data.entry(chat_id).or_insert_with(UserStats::default);
        if stats.is_premium != status {
            stats.is_premium = status;
            log::info!("User {} premium status set to {}", chat_id, status);
            true // Status changed
        } else {
            false // Status was already as requested
        }
    }

    // NEW METHOD to get user status (optional, but good for confirmation messages)
    pub async fn get_user_stats(&self, chat_id: i64) -> Option<UserStats> {
        let data = self.usage_data.lock().await;
        data.get(&chat_id).cloned()
    }

    // NEW METHOD: Get a summary of all user usage data for admin display
    pub async fn get_all_usage_summary(&self) -> io::Result<HashMap<i64, UserStats>> {
        let data = self.usage_data.lock().await;
        Ok(data.clone()) // Clone the HashMap to send it out, allowing the lock to be released
    }
}