use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::info;
use reqwest::Client;
use serde_json::Value;

pub struct StudioScraper {
    pub download_dir: PathBuf,
    pub pexels_key: String,
    pub pixabay_key: String,
}

impl StudioScraper {
    pub fn new(download_dir: PathBuf, pexels_key: String, pixabay_key: String) -> Self {
        Self { download_dir, pexels_key, pixabay_key }
    }

    /// Fetches background videos or photos matching search keyword
    pub async fn fetch_media(&self, keyword: &str, source: &str, media_type: &str) -> Result<PathBuf> {
        let folder = self.download_dir.join(keyword.replace(" ", "_"));
        tokio::fs::create_dir_all(&folder).await?;

        info!("Fetching {} media for keyword '{}' from source '{}'", media_type, keyword, source);

        match source {
            "pexels" if !self.pexels_key.is_empty() => self.fetch_pexels(keyword, media_type, &folder).await,
            "pixabay" if !self.pixabay_key.is_empty() => self.fetch_pixabay(keyword, media_type, &folder).await,
            _ => self.fetch_pinterest(keyword, media_type, &folder).await,
        }
    }

    async fn fetch_pexels(&self, keyword: &str, media_type: &str, folder: &Path) -> Result<PathBuf> {
        let client = Client::new();
        let is_video = media_type == "video";
        let url = if is_video {
            format!("https://api.pexels.com/videos/search?query={}&per_page=1", keyword)
        } else {
            format!("https://api.pexels.com/v1/search?query={}&per_page=1", keyword)
        };

        let resp: Value = client.get(&url)
            .header("Authorization", &self.pexels_key)
            .send()
            .await?
            .json()
            .await?;

        let media_url = if is_video {
            resp.pointer("/videos/0/video_files/0/link").and_then(|v| v.as_str())
        } else {
            resp.pointer("/photos/0/src/large2x").and_then(|v| v.as_str())
        };

        if let Some(m_url) = media_url {
            let out_file = folder.join(if is_video { "clip.mp4" } else { "photo.jpg" });
            let bytes = client.get(m_url).send().await?.bytes().await?;
            tokio::fs::write(&out_file, bytes).await?;
            Ok(out_file)
        } else {
            self.fetch_pinterest(keyword, media_type, folder).await
        }
    }

    async fn fetch_pixabay(&self, keyword: &str, media_type: &str, folder: &Path) -> Result<PathBuf> {
        let client = Client::new();
        let is_video = media_type == "video";
        let url = if is_video {
            format!("https://pixabay.com/api/videos/?key={}&q={}&per_page=3", self.pixabay_key, keyword)
        } else {
            format!("https://pixabay.com/api/?key={}&q={}&per_page=3", self.pixabay_key, keyword)
        };

        let resp: Value = client.get(&url).send().await?.json().await?;

        let media_url = if is_video {
            resp.pointer("/hits/0/videos/medium/url").and_then(|v| v.as_str())
        } else {
            resp.pointer("/hits/0/largeImageURL").and_then(|v| v.as_str())
        };

        if let Some(m_url) = media_url {
            let out_file = folder.join(if is_video { "clip.mp4" } else { "photo.jpg" });
            let bytes = client.get(m_url).send().await?.bytes().await?;
            tokio::fs::write(&out_file, bytes).await?;
            Ok(out_file)
        } else {
            self.fetch_pinterest(keyword, media_type, folder).await
        }
    }

    async fn fetch_pinterest(&self, keyword: &str, media_type: &str, folder: &Path) -> Result<PathBuf> {
        info!("Executing yt-dlp Pinterest search for '{}'", keyword);
        let is_video = media_type == "video";
        let out_file = folder.join(if is_video { "clip.mp4" } else { "photo.jpg" });

        if out_file.exists() {
            return Ok(out_file);
        }

        let search_url = format!("https://www.pinterest.com/search/{}/?q={}", if is_video { "videos" } else { "pins" }, keyword);
        
        let _ = Command::new("yt-dlp")
            .arg("-o")
            .arg(&out_file)
            .arg("--no-playlist")
            .arg(&search_url)
            .status();

        if out_file.exists() {
            Ok(out_file)
        } else {
            anyhow::bail!("Failed to download media from Pinterest for keyword '{}'", keyword)
        }
    }
}
