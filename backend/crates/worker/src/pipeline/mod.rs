pub mod download;
pub mod transcribe;
pub mod analyze;
pub mod dedup;
pub mod clip;
pub mod caption;
pub mod crop;
pub mod reframe;
pub mod originality;
pub mod translate;
pub mod studio_llm;
pub mod tts;
pub mod scraper;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViralityScore {
    pub hook_score: i32,
    pub engagement_score: i32,
    pub value_score: i32,
    pub shareability_score: i32,
    pub total_score: i32,
    pub hook_type: String,
    pub virality_reasoning: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptSegment {
    pub start_time: String,
    pub end_time: String,
    pub text: String,
    pub relevance_score: f64,
    pub reasoning: String,
    pub hook_title: Option<String>,
    pub virality: ViralityScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptAnalysis {
    pub most_relevant_segments: Vec<TranscriptSegment>,
    pub summary: String,
    pub key_topics: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub task_id: uuid::Uuid,
    pub url: String,
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
    pub filtered_words: Vec<String>,
    pub auto_vertical_reframe: bool,
    pub reframe_preset: String,
    pub reframe_frame_skip: u32,
    pub reframe_layout: String,
    pub speaker_active_switch: bool,
    pub split_divider: bool,
    pub originality_boost: String,
    pub translate_language: String,
    pub giphy_api_key: Option<String>,
    pub output_dir: String,
    pub temp_dir: String,
    pub gemini_api_key: String,
    pub gemini_model: String,
    pub deepgram_api_key: String,
    pub stt_provider: String,
    pub vosk_model_path: String,
    pub whisper_model_path: String,
    pub pyannote_segmentation_model_path: String,
    pub pyannote_embedding_model_path: String,
    pub pexels_api_key: Option<String>,
    pub pixabay_api_key: Option<String>,
    pub studio_payload: Option<serde_json::Value>,
    pub highlight_color: String,
    pub caption_animation: String,
    pub auto_emojis: bool,
    pub watermark_position: String,
    pub watermark_opacity: f64,
    pub watermark_path: Option<String>,
}
