use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tracing::{info, warn};

const WAVESPEED_BASE_URL: &str = "https://api.wavespeed.ai";

fn wavespeed_video_model() -> String {
    std::env::var("WAVESPEED_VIDEO_MODEL")
        .unwrap_or_else(|_| "bytedance/seedance-v1-pro-fast/text-to-video".into())
}

fn wavespeed_video_resolution() -> String {
    std::env::var("WAVESPEED_VIDEO_RESOLUTION").unwrap_or_else(|_| "480p".into())
}

#[derive(Debug, Clone)]
pub struct WavespeedTask {
    pub id: Option<String>,
    pub get_url: Option<String>,
    pub status: Option<String>,
    pub outputs: Vec<String>,
    pub error: Option<String>,
}

/// Lightweight client for the WaveSpeed API (text-to-video + music).
/// Ported from the Remotion text-to-video generator's `WavespeedClient`.
pub struct WavespeedClient {
    api_key: String,
}

impl WavespeedClient {
    pub fn new(api_key: String) -> Self {
        Self { api_key }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(v) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", self.api_key)) {
            headers.insert(reqwest::header::AUTHORIZATION, v);
        }
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers
    }

    async fn request_json(
        &self,
        client: &reqwest::Client,
        url: &str,
        data: Option<&serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut builder = client.get(url).headers(self.headers());
        if let Some(body) = data {
            builder = client.post(url).headers(self.headers()).json(body);
        }
        let response = builder
            .send()
            .await
            .context("WaveSpeed request failed")?;
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("WaveSpeed API error {}: {}", status, text);
        }
        Ok(response.json().await.context("WaveSpeed JSON parse failed")?)
    }

    fn parse_task(body: &serde_json::Value) -> WavespeedTask {
        let task = body.get("data").unwrap_or(body);
        WavespeedTask {
            id: task.get("id").and_then(|v| v.as_str()).map(String::from),
            get_url: task
                .pointer("/urls/get")
                .and_then(|v| v.as_str())
                .map(String::from),
            status: task.get("status").and_then(|v| v.as_str()).map(String::from),
            outputs: task
                .get("outputs")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            error: task.get("error").and_then(|v| v.as_str()).map(String::from),
        }
    }

    /// Submit a job to any WaveSpeed model endpoint and return the result URL.
    async fn submit(
        &self,
        client: &reqwest::Client,
        model: &str,
        payload: serde_json::Value,
    ) -> Result<String> {
        let url = format!("{}/api/v3/{}", WAVESPEED_BASE_URL, model);
        info!(
            "[WaveSpeed] Requesting {}: {}",
            model,
            serde_json::to_string(&payload).unwrap_or_default().chars().take(200).collect::<String>()
        );
        let body = self.request_json(client, &url, Some(&payload)).await?;
        let task = Self::parse_task(&body);
        let result_url = task
            .get_url
            .clone()
            .unwrap_or_else(|| format!("{}/api/v3/{}/result", WAVESPEED_BASE_URL, task.id.clone().unwrap_or_default()));
        if result_url.contains("result") && task.id.is_none() && task.get_url.is_none() {
            anyhow::bail!(
                "WaveSpeed submission response did not contain a prediction id: {}",
                body
            );
        }
        Ok(result_url)
    }

    /// Trigger a text-to-video generation (Seedance). Duration clamped to 3-10s.
    pub async fn trigger_video(
        &self,
        client: &reqwest::Client,
        prompt: &str,
        aspect_ratio: &str,
        duration_sec: i32,
    ) -> Result<String> {
        let model = wavespeed_video_model();
        let resolution = wavespeed_video_resolution();
        let duration = duration_sec.clamp(3, 10);
        self.submit(
            client,
            &model,
            serde_json::json!({
                "prompt": prompt,
                "aspect_ratio": aspect_ratio,
                "resolution": resolution,
                "duration": duration,
            }),
        )
        .await
    }

    /// Trigger WaveSpeed Lyria background music generation.
    pub async fn trigger_music(
        &self,
        client: &reqwest::Client,
        prompt: &str,
    ) -> Result<String> {
        self.submit(
            client,
            "google/lyria-3-clip/music",
            serde_json::json!({ "prompt": prompt }),
        )
        .await
    }

    /// Poll a prediction result URL until the job completes, returns output URLs.
    pub async fn poll_prediction(
        &self,
        client: &reqwest::Client,
        result_url: &str,
        timeout_secs: u64,
    ) -> Result<Vec<String>> {
        let start = std::time::Instant::now();
        while start.elapsed().as_secs() < timeout_secs {
            let body = self.request_json(client, result_url, None).await?;
            let task = Self::parse_task(&body);
            match task.status.as_deref() {
                Some("completed") => {
                    if task.outputs.is_empty() {
                        anyhow::bail!(
                            "WaveSpeed prediction completed but returned no outputs: {}",
                            body
                        );
                    }
                    return Ok(task.outputs);
                }
                Some("failed") | Some("cancelled") | Some("timeout") => {
                    anyhow::bail!(
                        "WaveSpeed prediction failed: {}",
                        task.error.unwrap_or_else(|| "unknown error".into())
                    );
                }
                Some(s) if s != "created" && s != "processing" => {
                    anyhow::bail!("Unexpected WaveSpeed status: {}", s);
                }
                _ => {}
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        anyhow::bail!("WaveSpeed prediction timed out after {}s", timeout_secs);
    }
}

/// Download a remote file to a local path (used to fetch AI clips/music locally).
pub async fn download_to_file(url: &str, dest: &Path) -> Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to download {}", url))?;
    if !response.status().is_success() {
        anyhow::bail!("Download failed {}: {}", response.status(), url);
    }
    let bytes = response.bytes().await.context("Failed to read download body")?;
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(dest, &bytes)
        .await
        .context("Failed to write downloaded file")?;
    info!("Downloaded {} bytes to {}", bytes.len(), dest.display());
    Ok(())
}

/// The SoundHelix tracks used as a fallback when Lyria is unavailable.
const SOUNDHELIX_FALLBACKS: &[&str] = &[
    "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3",
    "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-2.mp3",
    "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-3.mp3",
    "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-4.mp3",
    "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-5.mp3",
];

/// Generate background music via WaveSpeed Lyria, downloading the result locally.
/// Falls back to a SoundHelix track if Lyria is unavailable.
pub async fn generate_background_music(
    api_key: &str,
    topic: &str,
    temp_dir: &Path,
) -> Result<PathBuf> {
    let client = reqwest::Client::new();
    let ws = WavespeedClient::new(api_key.to_string());
    let dest = temp_dir.join(format!("music_lyria_{}.mp3", uuid::Uuid::new_v4()));

    match ws.trigger_music(&client, &format!("instrumental cinematic background music inspired by {}", topic)).await {
        Ok(result_url) => match ws.poll_prediction(&client, &result_url, 600).await {
            Ok(outputs) => {
                if let Some(url) = outputs.first() {
                    download_to_file(url, &dest).await?;
                    return Ok(dest);
                }
                anyhow::bail!("Lyria returned no outputs");
            }
            Err(e) => {
                warn!("[WaveSpeed] Lyria music failed: {} — falling back to SoundHelix", e);
            }
        },
        Err(e) => {
            warn!("[WaveSpeed] Lyria music request failed: {} — falling back to SoundHelix", e);
        }
    }

    // SoundHelix fallback
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let track = SOUNDHELIX_FALLBACKS[(nanos as usize) % SOUNDHELIX_FALLBACKS.len()];
    download_to_file(track, &dest).await?;
    Ok(dest)
}

/// Generate a WaveSpeed Seedance B-roll clip for a visual prompt, downloading it
/// locally. Returns a local path. Errors bubble up so the caller can fall back.
pub async fn generate_ai_clip(
    api_key: &str,
    prompt: &str,
    aspect_ratio: &str,
    duration_sec: i32,
    dest_dir: &Path,
) -> Result<PathBuf> {
    let client = reqwest::Client::new();
    let ws = WavespeedClient::new(api_key.to_string());
    let result_url = ws
        .trigger_video(&client, prompt, aspect_ratio, duration_sec)
        .await?;
    let outputs = ws.poll_prediction(&client, &result_url, 600).await?;
    let url = outputs
        .first()
        .ok_or_else(|| anyhow::anyhow!("Seedance clip returned no output"))?;
    let dest = dest_dir.join(format!("ai_clip_{}.mp4", uuid::Uuid::new_v4()));
    download_to_file(url, &dest).await?;
    Ok(dest)
}

/// Generate one Seedance clip per sentence (in parallel, concurrency-limited),
/// falling back to a Pexels stock clip when a generation fails. Returns local
/// media paths aligned with the sentence list (None entries are skipped).
pub async fn generate_ai_media_paths(
    api_key: String,
    prompts: Vec<String>,
    aspect_ratio: String,
    clip_duration_sec: i32,
    dest_dir: PathBuf,
    pexels_key: String,
    keywords: Vec<String>,
) -> Vec<Option<PathBuf>> {
    use futures::stream::{self, StreamExt};

    let client = reqwest::Client::new();
    let ws = WavespeedClient::new(api_key);
    let concurrency = std::env::var("WAVESPEED_CLIP_CONCURRENCY")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(3);

    let results: Vec<Option<PathBuf>> = stream::iter(prompts.into_iter().enumerate())
        .map(|(idx, prompt)| {
            let ws = &ws;
            let client = &client;
            let dest_dir = dest_dir.clone();
            let aspect_ratio = aspect_ratio.clone();
            let keywords = keywords.clone();
            let pexels_key = pexels_key.clone();
            async move {
                let dest = dest_dir.join(format!("ai_clip_{:03}.mp4", idx + 1));
                let result_url = ws
                    .trigger_video(client, &prompt, &aspect_ratio, clip_duration_sec)
                    .await;
                let outputs = match result_url {
                    Ok(u) => ws.poll_prediction(client, &u, 600).await,
                    Err(e) => Err(e),
                };
                match outputs {
                    Ok(urls) => match urls.first() {
                        Some(url) => {
                            if let Err(e) = download_to_file(url, &dest).await {
                                warn!("[WaveSpeed] clip {} download failed: {}", idx + 1, e);
                                None
                            } else {
                                Some(dest)
                            }
                        }
                        None => None,
                    },
                    Err(e) => {
                        warn!("[WaveSpeed] clip {} generation failed: {} — trying Pexels", idx + 1, e);
                        // Fall back to a stock clip via the scraper
                        let keyword = keywords.get(idx).cloned().unwrap_or(prompt);
                        match crate::pipeline::scraper::StudioScraper::new(
                            dest_dir.clone(),
                            pexels_key.clone(),
                            String::new(),
                        )
                        .fetch_media(&keyword, "stock_api", "video")
                        .await
                        {
                            Ok(p) => Some(p),
                            Err(_) => None,
                        }
                    }
                }
            }
        })
        .buffered(concurrency)
        .collect()
        .await;

    results
}

