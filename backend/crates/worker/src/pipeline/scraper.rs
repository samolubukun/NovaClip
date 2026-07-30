use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;
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
            "pinterest" => self.fetch_pinterest(keyword, media_type, &folder).await,
            "stock_api" => {
                if !self.pexels_key.is_empty() {
                    if let Ok(res) = self.fetch_pexels(keyword, media_type, &folder).await {
                        return Ok(res);
                    }
                }
                if !self.pixabay_key.is_empty() {
                    if let Ok(res) = self.fetch_pixabay(keyword, media_type, &folder).await {
                        return Ok(res);
                    }
                }
                anyhow::bail!("Pexels & Pixabay both failed or have no keys set");
            }
            _ => {
                // "all" — try Pexels → Pixabay → Pinterest
                if !self.pexels_key.is_empty() {
                    if let Ok(res) = self.fetch_pexels(keyword, media_type, &folder).await {
                        return Ok(res);
                    }
                }
                if !self.pixabay_key.is_empty() {
                    if let Ok(res) = self.fetch_pixabay(keyword, media_type, &folder).await {
                        return Ok(res);
                    }
                }
                self.fetch_pinterest(keyword, media_type, &folder).await
            }
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
        info!("Executing Pinterest scraper for keyword '{}'", keyword);
        let is_video = media_type == "video";
        let out_file = folder.join(if is_video { "clip.mp4" } else { "photo.jpg" });

        if out_file.exists() {
            return Ok(out_file);
        }

        // Call the Python Pinterest scraper script
        let script_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("pinterest_scraper.py");

        // Try common locations for the script
        let script = if script_path.exists() {
            script_path
        } else {
            let alt = PathBuf::from("pinterest_scraper.py");
            if alt.exists() { alt }
            else {
                anyhow::bail!("pinterest_scraper.py not found — cannot scrape Pinterest");
            }
        };

        let mt = if is_video { "video" } else { "photo" };

        let venv_python = PathBuf::from("novaclip_reframe").join("venv").join("Scripts").join("python.exe");
        let python_bin = if venv_python.exists() { venv_python } else { PathBuf::from("python") };

        let output = Command::new(&python_bin)
            .arg(script.to_str().unwrap())
            .arg(keyword)
            .arg(mt)
            .arg(folder.to_str().unwrap())
            .output()
            .await
            .context("Failed to run Pinterest scraper script")?;

        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                let result = PathBuf::from(&path_str);
                if result.exists() {
                    if std::fs::rename(&result, &out_file).is_err() {
                        let _ = std::fs::copy(&result, &out_file);
                    }
                    if out_file.exists() {
                        return Ok(out_file);
                    }
                }
            }
        }

        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Pinterest scraper failed for '{}': {}", keyword, stderr.trim())
    }
}
