use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub status: String,
    pub progress: i32,
    pub progress_message: Option<String>,
    pub source_url: String,
    pub source_title: Option<String>,
    pub source_type: String,
    pub aspect_ratio: String,
    pub num_clips: i32,
    pub font_family: String,
    pub font_size: i32,
    pub font_color: String,
    pub caption_template: String,
    pub add_subtitles: bool,
    pub include_broll: bool,
    pub processing_mode: String,
    pub cut_long_pauses: bool,
    pub pause_threshold_ms: i32,
    pub remove_filler_words: bool,
    /// JSON-encoded Vec<String>, stored as TEXT in SQLite
    pub filtered_words: String,
    pub transcript_cache: Option<String>,
    /// JSON-encoded map, stored as TEXT in SQLite
    pub stage_timings: String,
    pub error_message: Option<String>,
    pub gemini_api_key: Option<String>,
    pub deepgram_api_key: Option<String>,
    pub cancel_requested: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct GeneratedClip {
    pub id: String,
    pub task_id: String,
    pub clip_order: i32,
    pub filename: String,
    pub file_path: String,
    pub start_time: String,
    pub end_time: String,
    pub duration: f64,
    pub transcript_text: Option<String>,
    pub relevance_score: f64,
    pub virality_score: i32,
    pub hook_score: i32,
    pub engagement_score: i32,
    pub value_score: i32,
    pub shareability_score: i32,
    pub hook_type: Option<String>,
    pub hook_title: Option<String>,
    pub reasoning: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTask {
    pub source_url: String,
    pub source_type: String,
    pub source_title: Option<String>,
    pub aspect_ratio: String,
    pub num_clips: i32,
    pub font_family: String,
    pub font_size: i32,
    pub font_color: String,
    pub caption_template: String,
    pub add_subtitles: bool,
    pub include_broll: bool,
    pub processing_mode: String,
    pub cut_long_pauses: bool,
    pub pause_threshold_ms: i32,
    pub remove_filler_words: bool,
    pub filtered_words: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: Uuid,
    pub status: String,
    pub progress: i32,
    pub progress_message: Option<String>,
    pub source_url: String,
    pub source_title: Option<String>,
    pub source_type: String,
    pub clips_count: i64,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
