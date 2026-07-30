use std::env;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub cors_origins: Vec<String>,
    pub output_dir: String,
    pub temp_dir: String,
    pub max_video_upload_bytes: u64,
    pub gemini_api_key: String,
    pub deepgram_api_key: String,
    pub gemini_model: String,
    pub pexels_api_key: Option<String>,
    pub pixabay_api_key: Option<String>,
    pub spaces_endpoint: Option<String>,
    pub spaces_key: Option<String>,
    pub spaces_secret: Option<String>,
    pub spaces_bucket: Option<String>,
    pub spaces_public_url: Option<String>,
    pub max_concurrent_jobs: usize,
    pub job_timeout_seconds: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite://novaclip.db".into()),
            cors_origins: env::var("CORS_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:5173,http://localhost:3000".into())
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            output_dir: env::var("OUTPUT_DIR").unwrap_or_else(|_| "outputs".into()),
            temp_dir: env::var("TEMP_DIR").unwrap_or_else(|_| "temp".into()),
            max_video_upload_bytes: env::var("MAX_VIDEO_UPLOAD_BYTES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1_000_000_000),
            gemini_api_key: env::var("GEMINI_API_KEY").unwrap_or_default(),
            deepgram_api_key: env::var("DEEPGRAM_API_KEY").unwrap_or_default(),
            gemini_model: env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "gemini-3.1-flash-lite".into()),
            pexels_api_key: env::var("PEXELS_API_KEY").ok().filter(|s| !s.is_empty()),
            pixabay_api_key: env::var("PIXABAY_API_KEY").ok().filter(|s| !s.is_empty()),
            spaces_endpoint: env::var("SPACES_ENDPOINT").ok().filter(|s| !s.is_empty()),
            spaces_key: env::var("SPACES_KEY").ok().filter(|s| !s.is_empty()),
            spaces_secret: env::var("SPACES_SECRET").ok().filter(|s| !s.is_empty()),
            spaces_bucket: env::var("SPACES_BUCKET_NAME").ok().filter(|s| !s.is_empty()),
            spaces_public_url: env::var("SPACES_PUBLIC_URL").ok().filter(|s| !s.is_empty()),
            max_concurrent_jobs: env::var("MAX_CONCURRENT_JOBS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4),
            job_timeout_seconds: env::var("JOB_TIMEOUT_SECONDS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10800),
        })
    }
}
