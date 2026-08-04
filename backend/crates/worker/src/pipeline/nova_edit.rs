use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use base64::Engine;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{info, warn};

use novaclip_db::DbPool;

use crate::{check_cancelled, emit_progress};
use crate::pipeline::crop::output_dimensions;
use crate::pipeline::download::{extract_audio, get_video_duration, resolve_upload_path};
use crate::pipeline::transcribe::{transcribe_audio, DeepgramWord, TimestampedTranscript};
use crate::PipelineConfig;

// ---------------------------------------------------------------------------
// Schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEditBrief {
    #[serde(default)]
    pub product: String,
    #[serde(default)]
    pub audience: String,
    #[serde(default)]
    pub tone: String,
    #[serde(default)]
    pub duration_seconds: i32,
    #[serde(default)]
    pub content_type: String,
    #[serde(default)]
    pub instruction: String,
    #[serde(default)]
    pub style_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NovaEditApiKeys {
    #[serde(default)]
    pub gemini_key: Option<String>,
    #[serde(default)]
    pub openrouter_key: Option<String>,
    #[serde(default)]
    pub deepgram_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEditPayload {
    pub brief: NovaEditBrief,
    #[serde(default)]
    pub footage: Vec<String>,
    #[serde(default)]
    pub api_keys: NovaEditApiKeys,
    #[serde(default)]
    pub llm_provider: String,
    /// "director" | "editing"
    #[serde(default)]
    pub stage: String,
    #[serde(default)]
    pub retries_used: i32,
    #[serde(default)]
    pub max_retries: i32,
    #[serde(default)]
    pub review_threshold: f64,
    #[serde(default)]
    pub feedback_history: Vec<String>,
    #[serde(default)]
    pub user_feedback: Option<String>,
    #[serde(default)]
    pub visual_analysis: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEditPlanEntry {
    pub shot_id: String,
    pub start_trim: f64,
    pub end_trim: f64,
    pub position: i32,
    #[serde(default)]
    pub text_overlay: Option<String>,
    #[serde(default)]
    pub transition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEditPlan {
    pub brief: Value,
    pub entries: Vec<NovaEditPlanEntry>,
    pub total_duration: f64,
    #[serde(default)]
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEditReview {
    #[serde(default)]
    pub adherence: f64,
    #[serde(default)]
    pub pacing: f64,
    #[serde(default)]
    pub visual_quality: f64,
    #[serde(default)]
    pub watchability: f64,
    #[serde(default)]
    pub overall: f64,
    #[serde(default)]
    pub feedback: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaShot {
    pub source_file: String,
    pub start_time: f64,
    pub end_time: f64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub energy_level: i32,
    #[serde(default)]
    pub relevance_score: f64,
    #[serde(default)]
    pub transcript: String,
    #[serde(default)]
    pub words: Vec<DeepgramWord>,
    #[serde(default)]
    pub roll_type: String,
    #[serde(default)]
    pub visual_description: String,
    #[serde(default)]
    pub visual_quality: f64,
    #[serde(default)]
    pub shot_type: String,
}

const FOOTAGE_INDEX: &str = "footage_index.json";
const DIRECTOR_DEFAULT_MODEL: &str = "gemini-3.1-flash-lite";

// ---------------------------------------------------------------------------
// Public entry point — dispatched from lib.rs for source_type == "agentic"
// ---------------------------------------------------------------------------

pub async fn process_nova_edit_task(
    db: &DbPool,
    cfg: &PipelineConfig,
    output_dir: &Path,
    task_id_str: &str,
) -> Result<()> {
    let mut payload: NovaEditPayload = cfg
        .novaedit_payload
        .clone()
        .and_then(|v| serde_json::from_value(v).ok())
        .ok_or_else(|| anyhow::anyhow!("No novaedit_payload found for agentic task"))?;

    if payload.stage.is_empty() {
        payload.stage = "director".to_string();
        save_payload(db, task_id_str, &payload).await?;
    }

    info!("NovaEdit task {} — stage: {}", task_id_str, payload.stage);

    match payload.stage.as_str() {
        "director" => run_director_stage(db, cfg, &mut payload, output_dir, task_id_str).await,
        "editing" => run_editing_stage(db, cfg, &mut payload, output_dir, task_id_str).await,
        other => anyhow::bail!("Unknown NovaEdit stage: {}", other),
    }
}

// ---------------------------------------------------------------------------
// Stage 1 — Director: preprocess footage, transcribe, plan the edit
// ---------------------------------------------------------------------------

async fn run_director_stage(
    db: &DbPool,
    cfg: &PipelineConfig,
    payload: &mut NovaEditPayload,
    output_dir: &Path,
    task_id_str: &str,
) -> Result<()> {
    emit_progress(db, task_id_str, 5, "Preprocessing footage — scene detection & transcription...", "processing").await;
    let shots = preprocess_footage(db, cfg, output_dir, payload, task_id_str).await?;
    if check_cancelled(db, task_id_str).await {
        return Ok(());
    }
    save_index(output_dir, &shots).await?;

    emit_progress(db, task_id_str, 22, "Building packed transcript...", "processing").await;
    let packed = pack_transcript(&shots, &payload.brief);

    emit_progress(db, task_id_str, 30, "Director agent is planning your edit...", "processing").await;
    let mut plan = run_director(cfg, payload, &packed, &payload.feedback_history, payload.user_feedback.as_deref()).await?;
    plan = sanitize_plan(plan, &shots, &payload.brief);

    save_edit_plan(db, task_id_str, &plan).await?;
    payload.stage = "editing".to_string();
    save_payload(db, task_id_str, payload).await?;

    sqlx::query(
        "UPDATE tasks SET status = 'awaiting_approval', progress = 45, progress_message = 'Edit plan ready — review & approve to render', updated_at = datetime('now') WHERE id = ?"
    )
    .bind(task_id_str)
    .execute(db)
    .await?;

    info!("NovaEdit {} — edit plan produced ({} entries, {:.1}s), awaiting approval", task_id_str, plan.entries.len(), plan.total_duration);
    Ok(())
}

// ---------------------------------------------------------------------------
// Stage 2 — Editing: render → review → feedback loop
// ---------------------------------------------------------------------------

async fn run_editing_stage(
    db: &DbPool,
    cfg: &PipelineConfig,
    payload: &mut NovaEditPayload,
    output_dir: &Path,
    task_id_str: &str,
) -> Result<()> {
    let mut shots = load_index(output_dir).await?;
    if shots.is_empty() {
        anyhow::bail!("Footage index missing — re-run the Director stage");
    }

    let mut version: i32 = 0;

    loop {
        if check_cancelled(db, task_id_str).await {
            return Ok(());
        }

        // User requested a re-plan (manual feedback after completion)
        if version == 0 && payload.user_feedback.is_some() {
            let feedback = payload.user_feedback.clone().unwrap_or_default();
            emit_progress(db, task_id_str, 50, "Re-planning with your feedback...", "processing").await;
            let packed = pack_transcript(&shots, &payload.brief);
            let mut plan = run_director(cfg, payload, &packed, &[feedback.clone()], None).await?;
            plan = sanitize_plan(plan, &shots, &payload.brief);
            save_edit_plan(db, task_id_str, &plan).await?;
            payload.user_feedback = None;
            save_payload(db, task_id_str, payload).await?;
        }

        let plan: NovaEditPlan = read_edit_plan(db, task_id_str).await?;
        if plan.entries.is_empty() {
            anyhow::bail!("Empty edit plan — cannot render");
        }

        let msg = if version == 0 {
            "Editor is rendering your cut...".to_string()
        } else {
            format!("Editor is re-rendering (attempt {})...", version + 1)
        };
        emit_progress(db, task_id_str, 55, &msg, "processing").await;
        let final_path = run_editor(cfg, output_dir, &plan, &mut shots).await?;

        emit_progress(db, task_id_str, 78, "Reviewer agent is scoring your video...", "processing").await;
        let review = run_reviewer(cfg, payload, &plan, &shots).await?;

        sqlx::query(
            "UPDATE tasks SET review_score = ?, updated_at = datetime('now') WHERE id = ?"
        )
        .bind(serde_json::to_string(&review).unwrap_or_default())
        .bind(task_id_str)
        .execute(db)
        .await?;

        info!(
            "NovaEdit {} — review attempt {}: overall={:.2} (feedback={})",
            task_id_str, version + 1, review.overall, review.feedback
        );

        payload.retries_used += 1;
        let should_retry = review.overall < payload.review_threshold
            && (payload.retries_used as i32) < payload.max_retries;

        if !should_retry {
            save_payload(db, task_id_str, payload).await?;
            finalize_clip(db, output_dir, &final_path, task_id_str, &plan).await?;
            emit_progress(db, task_id_str, 100, "Agentic edit complete!", "completed").await;
            return Ok(());
        }

        payload.feedback_history.push(review.feedback.clone());
        save_payload(db, task_id_str, payload).await?;

        emit_progress(
            db,
            task_id_str,
            88,
            &format!(
                "Reviewer scored {:.2} — Director is re-planning with feedback (attempt {}/{})",
                review.overall,
                payload.retries_used,
                payload.max_retries
            ),
            "processing",
        ).await;

        let packed = pack_transcript(&shots, &payload.brief);
        let mut plan = run_director(cfg, payload, &packed, &payload.feedback_history, None).await?;
        plan = sanitize_plan(plan, &shots, &payload.brief);
        save_edit_plan(db, task_id_str, &plan).await?;
        shots = load_index(output_dir).await?;
        version += 1;
    }
}

// ---------------------------------------------------------------------------
// Preprocessing — scene detection + transcription → NovaShot[]
// ---------------------------------------------------------------------------

async fn preprocess_footage(
    db: &DbPool,
    cfg: &PipelineConfig,
    output_dir: &Path,
    payload: &NovaEditPayload,
    task_id_str: &str,
) -> Result<Vec<NovaShot>> {
    if payload.footage.is_empty() {
        anyhow::bail!("No footage uploaded for NovaEdit task");
    }

    let mut all_shots: Vec<NovaShot> = Vec::new();
    let total = payload.footage.len();

    for (i, upload_ref) in payload.footage.iter().enumerate() {
        let path = resolve_upload_path(upload_ref, &cfg.temp_dir);
        if !path.exists() {
            warn!("NovaEdit footage missing: {} — skipping", path.display());
            continue;
        }

        let pct = 5 + (i as i32 * 15 / total.max(1) as i32);
        emit_progress(
            db,
            task_id_str,
            pct,
            &format!("Detecting scenes in clip {} of {}...", i + 1, total),
            "processing",
        ).await;
        let scenes = detect_scenes(&path).await?;

        let transcript = match extract_audio(&path, &cfg.temp_dir).await {
            Ok(audio) => {
                let result = transcribe_audio(
                    &audio,
                    &cfg.stt_provider,
                    &cfg.deepgram_api_key,
                    Path::new(&cfg.vosk_model_path),
                    Path::new(&cfg.whisper_model_path),
                    Path::new(&cfg.pyannote_segmentation_model_path),
                    Path::new(&cfg.pyannote_embedding_model_path),
                ).await;
                let _ = std::fs::remove_file(&audio);
                result.unwrap_or(TimestampedTranscript { full_text: String::new(), words: Vec::new(), duration: 0.0 })
            }
            Err(err) => {
                warn!("NovaEdit visual-only footage: audio unavailable for {}: {}", path.display(), err);
                TimestampedTranscript { full_text: String::new(), words: Vec::new(), duration: 0.0 }
            }
        };

        for (s, e) in &scenes {
            let s = *s;
            let e = *e;
            let words = words_for_window(&transcript.words, s, e);
            let text = words_to_text(&words);
            all_shots.push(NovaShot {
                source_file: path.to_string_lossy().to_string(),
                start_time: s,
                end_time: e,
                description: String::new(),
                energy_level: 0,
                relevance_score: 0.0,
                transcript: text,
                words,
                roll_type: detect_roll_type(&path),
                visual_description: String::new(),
                visual_quality: 0.0,
                shot_type: String::new(),
            });
        }

        info!("NovaEdit {} — processed clip {}: {} scenes, {} words", task_id_str, i + 1, scenes.len(), transcript.words.len());
        let _ = output_dir; // output_dir reserved for future artifacts
    }

    if all_shots.is_empty() {
        anyhow::bail!("No usable footage found for NovaEdit task");
    }

    if payload.visual_analysis {
        analyze_visual_shots(cfg, output_dir, payload, &mut all_shots).await?;
    }

    Ok(all_shots)
}

async fn analyze_visual_shots(
    cfg: &PipelineConfig,
    output_dir: &Path,
    payload: &NovaEditPayload,
    shots: &mut [NovaShot],
) -> Result<()> {
    const VISION_MODELS: [&str; 3] = [
        "google/gemma-4-26b-a4b-it:free",
        "nvidia/nemotron-nano-12b-v2-vl:free",
        "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free",
    ];
    let selected = if payload.llm_provider.contains('/') { payload.llm_provider.as_str() } else { cfg.llm_provider.as_str() };
    let mut providers: Vec<&str> = if VISION_MODELS.contains(&selected) { vec![selected] } else { Vec::new() };
    providers.extend(VISION_MODELS.iter().copied().filter(|model| *model != selected));
    let api_key = payload.api_keys.openrouter_key.clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_default();
    if api_key.is_empty() || providers.is_empty() {
        warn!("NovaEdit visual analysis skipped: an OpenRouter vision model and key are required");
        return Ok(());
    }

    let visual_dir = output_dir.join("visual_frames");
    tokio::fs::create_dir_all(&visual_dir).await?;
    for (index, shot) in shots.iter().enumerate() {
        let frame = visual_dir.join(format!("frame_{index:04}.jpg"));
        let source = Path::new(&shot.source_file);
        let midpoint = (shot.start_time + shot.end_time) / 2.0;
        let status = Command::new("ffmpeg")
            .args(["-y", "-ss", &format!("{midpoint:.3}"), "-i", source.to_str().unwrap_or_default(), "-frames:v", "1", "-vf", "scale=512:-2", frame.to_str().unwrap_or_default()])
            .status().await?;
        if !status.success() { continue; }
    }

    let batch_size = 12usize;
    for batch_start in (0..shots.len()).step_by(batch_size) {
        let batch_end = (batch_start + batch_size).min(shots.len());
        let mut content = vec![json!({
            "type": "text",
            "text": format!("Analyze the numbered video frames. Return strict JSON only in the form {{\"frames\":[{{\"frame_id\":\"shot_0\",\"description\":\"...\",\"shot_type\":\"...\",\"visual_quality\":0.0}}]}}. Do not invent details.")
        })];
        for index in batch_start..batch_end {
            let frame = visual_dir.join(format!("frame_{index:04}.jpg"));
            if let Ok(bytes) = tokio::fs::read(&frame).await {
                let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
                content.push(json!({ "type": "text", "text": format!("Frame shot_{index}:" ) }));
                content.push(json!({ "type": "image_url", "image_url": { "url": format!("data:image/jpeg;base64,{encoded}") } }));
            }
        }
        let mut body: Option<Value> = None;
        for provider in &providers {
            let response = reqwest::Client::new().post("https://openrouter.ai/api/v1/chat/completions")
                .header("Authorization", format!("Bearer {api_key}"))
                .header("HTTP-Referer", "https://novaclip.app")
                .header("X-Title", "NovaClip")
                .json(&json!({ "model": provider, "messages": [{ "role": "user", "content": content }], "temperature": 0.1, "max_tokens": 2000 }))
                .send().await?;
            if response.status().is_success() {
                body = Some(response.json().await?);
                break;
            }
            warn!("NovaEdit visual model {} failed: {}", provider, response.status());
        }
        let Some(body) = body else { continue };
        let text = body.pointer("/choices/0/message/content").and_then(Value::as_str).unwrap_or("{}");
        let clean = strip_fences(text);
        let parsed: Value = match serde_json::from_str(&clean) { Ok(value) => value, Err(_) => continue };
        for item in parsed.get("frames").and_then(Value::as_array).into_iter().flatten() {
            let Some(id) = item.get("frame_id").and_then(Value::as_str) else { continue };
            let Ok(index) = id.trim_start_matches("shot_").parse::<usize>() else { continue };
            if let Some(shot) = shots.get_mut(index) {
                shot.visual_description = item.get("description").and_then(Value::as_str).unwrap_or_default().to_string();
                shot.shot_type = item.get("shot_type").and_then(Value::as_str).unwrap_or_default().to_string();
                shot.visual_quality = item.get("visual_quality").and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0);
            }
        }
    }
    Ok(())
}

async fn detect_scenes(video: &Path) -> Result<Vec<(f64, f64)>> {
    let dur = get_video_duration(video).await?.max(0.1);
    let output = Command::new("ffmpeg")
        .args([
            "-i", video.to_str().unwrap_or_default(),
            "-vf", "select='gt(scene,0.35)',showinfo",
            "-f", "null", "-",
        ])
        .output()
        .await
        .context("ffmpeg scene detection failed")?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut cuts: Vec<f64> = Vec::new();
    for line in stderr.lines() {
        if let Some(pos) = line.find("pts_time:") {
            let tail = &line[pos + "pts_time:".len()..];
            let t: String = tail.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
            if let Ok(v) = t.parse::<f64>() {
                if v > 0.15 && v < dur - 0.15 {
                    cuts.push(v);
                }
            }
        }
    }
    cuts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    cuts.dedup_by(|a, b| (*a - *b).abs() < 0.2);

    if cuts.is_empty() {
        return Ok(vec![(0.0, dur)]);
    }

    let mut shots = Vec::new();
    let mut prev = 0.0;
    for c in &cuts {
        shots.push((prev, *c));
        prev = *c;
    }
    shots.push((prev, dur));
    Ok(shots)
}

fn detect_roll_type(video: &Path) -> String {
    const PATTERNS: [&str; 8] = ["a-roll", "a_roll", "aroll", "a-rolls", "b-roll", "b_roll", "broll", "b-rolls"];
    if let Some(parent) = video.parent() {
        if let Some(name) = parent.file_name().and_then(|n| n.to_str()) {
            let lower = name.to_lowercase();
            for p in PATTERNS {
                if lower.contains(p) {
                    return if p.starts_with('a') { "a-roll".into() } else { "b-roll".into() };
                }
            }
        }
    }
    "unknown".into()
}

fn words_for_window(words: &[DeepgramWord], start: f64, end: f64) -> Vec<DeepgramWord> {
    words
        .iter()
        .filter(|w| {
            let mid = (w.start + w.end) / 2.0;
            mid >= start && mid <= end
        })
        .cloned()
        .collect()
}

fn words_to_text(words: &[DeepgramWord]) -> String {
    let mut out = String::new();
    for (i, w) in words.iter().enumerate() {
        let t = w.punctuated_word.as_deref().unwrap_or(&w.word);
        if i > 0 {
            let prev = words[i - 1].punctuated_word.as_deref().unwrap_or(&words[i - 1].word);
            let need_space = prev.chars().last().map(|c| !",.!?;:% )]}\"'".contains(c)).unwrap_or(true);
            let skip_lead = t.chars().next().map(|c| "([{$\"'#".contains(c)).unwrap_or(false);
            if need_space && !skip_lead {
                out.push(' ');
            }
        }
        out.push_str(t);
    }
    out.trim().to_string()
}

// ---------------------------------------------------------------------------
// Footage index persistence
// ---------------------------------------------------------------------------

async fn save_index(output_dir: &Path, shots: &[NovaShot]) -> Result<()> {
    let path = output_dir.join(FOOTAGE_INDEX);
    let data = serde_json::to_string_pretty(&json!({ "shots": shots }))?;
    tokio::fs::write(&path, data).await?;
    Ok(())
}

async fn load_index(output_dir: &Path) -> Result<Vec<NovaShot>> {
    let path = output_dir.join(FOOTAGE_INDEX);
    if !path.exists() {
        return Ok(vec![]);
    }
    let data = tokio::fs::read_to_string(&path).await?;
    let v: Value = serde_json::from_str(&data)?;
    Ok(v["shots"].as_array().map(|a| a.iter().filter_map(|s| serde_json::from_value(s.clone()).ok()).collect()).unwrap_or_default())
}

// ---------------------------------------------------------------------------
// Packed transcript (transcript-first reasoning, video-use style)
// ---------------------------------------------------------------------------

pub fn pack_transcript(shots: &[NovaShot], brief: &NovaEditBrief) -> String {
    let mut lines = String::new();
    lines.push_str("# Packed footage index — shot-level transcript\n\n");
    lines.push_str(&format!(
        "Creative brief: product={}, audience={}, tone={}, format={}, target duration={}s\n",
        if brief.product.is_empty() { "\"\"" } else { &brief.product },
        if brief.audience.is_empty() { "\"\"" } else { &brief.audience },
        if brief.tone.is_empty() { "\"natural\"" } else { &brief.tone },
        if brief.content_type.is_empty() { "short_form" } else { &brief.content_type },
        brief.duration_seconds
    ));
    if !brief.instruction.is_empty() {
        lines.push_str(&format!("Additional instruction: {}\n", brief.instruction));
    }
    lines.push('\n');

    for (idx, shot) in shots.iter().enumerate() {
        let stem = Path::new(&shot.source_file)
            .file_stem().map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("clip{}", idx));
        let sid = shot_id(&stem, shot.start_time);
        let d = (shot.end_time - shot.start_time).max(0.0);
        lines.push_str(&format!(
            "## shot: `{}`  (source: {} · {}, {:.1}s, {})\n",
            sid, stem, shot.roll_type, d, shot.words.len()
        ));
        lines.push_str("  [");
        lines.push_str(&format!("{:.2}s → {:.2}s", shot.start_time, shot.end_time));
        lines.push_str("] ");
        lines.push_str(if shot.transcript.trim().is_empty() { "_no speech detected_".into() } else { shot.transcript.trim().into() });
        if !shot.visual_description.is_empty() {
            lines.push_str(&format!("\n  Visual: {} (shot type: {}, quality: {:.2})", shot.visual_description, shot.shot_type, shot.visual_quality));
        }
        lines.push_str("\n\n");
    }
    lines
}

pub fn shot_id(stem: &str, start_time: f64) -> String {
    format!("{}#{:.2}", stem, start_time)
}

fn parse_shot_id(sid: &str) -> Option<(String, f64)> {
    let pos = sid.rfind('#')?;
    let stem = sid[..pos].to_string();
    let t: f64 = sid[pos + 1..].trim().parse().ok()?;
    Some((stem, t))
}

fn resolve_shot<'a>(shots: &'a [NovaShot], sid: &str) -> Option<&'a NovaShot> {
    let (stem, t) = parse_shot_id(sid)?;
    shots.iter().find(|s| {
        let s_stem = Path::new(&s.source_file).file_stem().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
        s_stem == stem && (s.start_time - t).abs() < 1e-3
    })
}

// ---------------------------------------------------------------------------
// Director agent (LLM) → EditPlan
// ---------------------------------------------------------------------------

async fn run_director(
    cfg: &PipelineConfig,
    payload: &NovaEditPayload,
    packed: &str,
    feedback: &[String],
    user_feedback: Option<&str>,
) -> Result<NovaEditPlan> {
    let brief = &payload.brief;
    let target = brief.duration_seconds.max(5);
    let is_long_form = brief.content_type == "long_form" || target > 120;
    let shot_guidance = if is_long_form {
        "Select enough shots to cover the full story, usually 10–120 shots depending on target duration and available footage. Organize the edit into a clear beginning, middle, and ending. Use chapter-like sections when the material supports them. Allow longer 8–45s segments when they preserve context, while removing dead air and repetition. Preserve chronological and conversational continuity unless the brief calls for a montage."
    } else {
        "Select 3–10 shots. Build a concise social-video arc with a strong hook, useful middle, and clear ending. Keep pacing tight, usually mixing 2–4s hooks with 5–10s segments."
    };

    let mut fb_block = String::new();
    if !feedback.is_empty() {
        fb_block.push_str("\n## Reviewer feedback from previous attempts — address every point:\n");
        for f in feedback {
            fb_block.push_str(&format!("- {}\n", f));
        }
    }
    if let Some(uf) = user_feedback {
        if !uf.trim().is_empty() {
            fb_block.push_str(&format!("\n## Human feedback — prioritize this:\n- {}\n", uf));
        }
    }

    let system_prompt = format!(
        r#"You are the DIRECTOR agent of an agentic video editor (NovaEdit). You select shots from a packed footage index and produce an EditPlan as a JSON object ONLY (no markdown, no prose).

CREATIVE BRIEF:
- Product/service: {product}
- Target audience: {audience}
- Tone: {tone}
- Target duration: {target}s (final cut must be within ±10% of this)
- Content format: {content_type}

RULES:
1. {shot_guidance} Prefer shots with real speech for A-roll. Use B-roll/visual shots for texture.
2. Each entry: shot_id MUST exactly match a shot id in the index (copy it verbatim). start_trim and end_trim are offsets IN SECONDS relative to that shot's start_time (start_trim < end_trim, within the shot's bounds). Pad each cut 0.1–0.3s past word boundaries to avoid clipped speech.
3. Build a narrative arc appropriate to the format: hook/context → development → payoff/conclusion (or adapt to the material). For long-form, preserve important context and continuity instead of forcing every section into a social-media hook.
4. Keep pacing varied and remove silence, repetition, and unusable footage without making speech feel rushed.
5. text_overlay: optional short on-screen text (max 40 chars) for a segment, or null.
6. position must be 0..N-1 contiguous in final order.

OUTPUT FORMAT (strict JSON):
{{"entries":[{{"shot_id":"...","start_trim":0.0,"end_trim":3.2,"position":0,"text_overlay":null}}],"rationale":"one short paragraph explaining the edit choices"}}"#,
        product = brief.product,
        audience = brief.audience,
        tone = brief.tone,
        target = target,
        content_type = if is_long_form { "long-form" } else { "short-form" },
        shot_guidance = shot_guidance,
    );

    let user_prompt = format!(
        "{}\n\nFOOTAGE INDEX:\n{}{}\n\nProduce the EditPlan now. Total cut length must land near {}s.",
        packed, fb_block, if !packed.contains("_no speech detected_") { "" } else { "" },
        target
    );

    let value = call_llm_json(cfg, payload, &system_prompt, &user_prompt).await?;
    let entries: Vec<NovaEditPlanEntry> = value
        .get("entries")
        .and_then(|e| e.as_array())
        .map(|a| a.iter().filter_map(|x| serde_json::from_value(x.clone()).ok()).collect())
        .unwrap_or_default();
    let rationale = value.get("rationale").and_then(|r| r.as_str()).unwrap_or_default().to_string();

    Ok(NovaEditPlan {
        brief: json!(brief),
        entries,
        total_duration: 0.0,
        rationale,
    })
}

fn sanitize_plan(mut plan: NovaEditPlan, shots: &[NovaShot], brief: &NovaEditBrief) -> NovaEditPlan {
    let mut kept: Vec<NovaEditPlanEntry> = Vec::new();
    for mut e in plan.entries {
        if e.end_trim <= e.start_trim {
            continue;
        }
        if let Some(shot) = resolve_shot(shots, &e.shot_id) {
            let shot_dur = shot.end_time - shot.start_time;
            e.start_trim = e.start_trim.clamp(0.0, shot_dur);
            e.end_trim = e.end_trim.clamp(0.0, shot_dur);
            if e.end_trim - e.start_trim < 0.4 {
                continue;
            }
            e.start_trim = snap_start(shot, e.start_trim);
            e.end_trim = snap_end(shot, e.end_trim);
            if e.end_trim - e.start_trim < 0.3 {
                continue;
            }
            kept.push(e);
        }
    }
    kept.sort_by_key(|e| e.position);
    for (i, e) in kept.iter_mut().enumerate() {
        e.position = i as i32;
    }
    let total: f64 = kept.iter().map(|e| e.end_trim - e.start_trim).sum();
    plan.entries = kept;
    plan.total_duration = total;
    plan.brief = json!(brief);
    plan
}

fn snap_start(shot: &NovaShot, t: f64) -> f64 {
    let mut best = t;
    let mut best_d = f64::MAX;
    for w in &shot.words {
        let d = (w.start - t).abs();
        if d < best_d {
            best_d = d;
            best = w.start;
        }
    }
    if best_d <= 0.5 { best.clamp(0.0, shot.end_time - shot.start_time) } else { t }
}

fn snap_end(shot: &NovaShot, t: f64) -> f64 {
    let mut best = t;
    let mut best_d = f64::MAX;
    for w in &shot.words {
        let d = (w.end - t).abs();
        if d < best_d {
            best_d = d;
            best = w.end;
        }
    }
    if best_d <= 0.5 { best.clamp(0.0, shot.end_time - shot.start_time) } else { t }
}

// ---------------------------------------------------------------------------
// Editor agent — deterministic FFmpeg rendering (hard rules applied)
// ---------------------------------------------------------------------------

async fn run_editor(
    cfg: &PipelineConfig,
    output_dir: &Path,
    plan: &NovaEditPlan,
    shots: &mut Vec<NovaShot>,
) -> Result<PathBuf> {
    let (out_w, out_h) = output_dimensions(&cfg.aspect_ratio);
    let mut segments: Vec<PathBuf> = Vec::new();
    let mut temp: Vec<PathBuf> = Vec::new();

    for e in &plan.entries {
        let shot = match resolve_shot(shots, &e.shot_id) {
            Some(s) => s,
            None => continue,
        };
        let src_start = shot.start_time + e.start_trim;
        let dur = (e.end_trim - e.start_trim).max(0.4);
        let out = output_dir.join(format!("seg_{:02}.mp4", e.position));
        let seg = render_segment(cfg, shot, src_start, dur, out_w, out_h, e.text_overlay.as_deref(), &out).await?;
        segments.push(seg.clone());
        temp.push(seg);
    }

    if segments.is_empty() {
        anyhow::bail!("Editor produced zero segments");
    }

    let concated = output_dir.join("novaedit_concat.mp4");
    concat_lossless(&segments, &concated).await?;
    temp.push(concated.clone());

    let final_video = output_dir.join("final_video.mp4");
    let renamed = tokio::fs::rename(&concated, &final_video).await;
    if renamed.is_err() {
        tokio::fs::copy(&concated, &final_video).await.ok();
    }

    for f in &temp {
        if f != &final_video {
            tokio::fs::remove_file(f).await.ok();
        }
    }

    info!("Editor rendered {} segments → {}", segments.len(), final_video.display());
    Ok(final_video)
}

async fn render_segment(
    cfg: &PipelineConfig,
    shot: &NovaShot,
    src_start: f64,
    dur: f64,
    out_w: u32,
    out_h: u32,
    text_overlay: Option<&str>,
    out: &Path,
) -> Result<PathBuf> {
    let fade_out_start = (dur - 0.03).max(0.0);

    let mut vf = format!(
        "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,setsar=1",
        out_w, out_h, out_w, out_h
    );

    let mut overlay_file: Option<PathBuf> = None;
    if let Some(txt) = text_overlay {
        if !txt.trim().is_empty() {
            let tf = out.with_extension("txt");
            tokio::fs::write(&tf, txt.trim()).await?;
            overlay_file = Some(tf.clone());
            let escaped = tf.to_str().unwrap_or_default().replace('\\', "/").replace(':', "\\:");
            let font_size = ((out_h as f64) * 0.05).round().max(36.0);
            vf.push_str(&format!(
                ",drawtext=textfile='{}':fontcolor=white:fontsize={:.0}:box=1:boxcolor=black@0.6:boxborderw=18:x=(w-text_w)/2:y=h-320",
                escaped, font_size
            ));
        }
    }

    let args = vec![
        "-y".to_string(),
        "-ss".to_string(), format!("{:.3}", src_start),
        "-i".to_string(), shot.source_file.clone(),
        "-t".to_string(), format!("{:.3}", dur),
        "-vf".to_string(), vf,
        "-af".to_string(), format!("afade=t=in:st=0:d=0.03,afade=t=out:st={:.3}:d=0.03", fade_out_start),
        "-c:v".to_string(), "libx264".to_string(),
        "-preset".to_string(), "fast".to_string(),
        "-crf".to_string(), "20".to_string(),
        "-pix_fmt".to_string(), "yuv420p".to_string(),
        "-c:a".to_string(), "aac".to_string(),
        "-b:a".to_string(), "192k".to_string(),
        "-ar".to_string(), "48000".to_string(),
        "-movflags".to_string(), "+faststart".to_string(),
        out.to_string_lossy().to_string(),
    ];

    let output = Command::new("ffmpeg")
        .args(&args)
        .output()
        .await
        .context("Failed to spawn ffmpeg for segment render")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Segment render failed: {}", stderr.trim());
    }
    if !out.exists() {
        anyhow::bail!("Segment render produced no file");
    }
    if let Some(tf) = overlay_file {
        let _ = std::fs::remove_file(tf);
    }
    let _ = cfg;
    Ok(out.to_path_buf())
}

async fn concat_lossless(segments: &[PathBuf], out: &Path) -> Result<()> {
    let list = out.with_extension("txt");
    let mut contents = String::new();
    for s in segments {
        let p = s.to_string_lossy().replace('\'', "'\\''");
        contents.push_str(&format!("file '{}'\n", p));
    }
    tokio::fs::write(&list, &contents).await?;

    let status = Command::new("ffmpeg")
        .args([
            "-y", "-f", "concat", "-safe", "0", "-i",
            list.to_str().unwrap_or_default(),
            "-c", "copy",
            "-movflags", "+faststart",
            out.to_str().unwrap_or_default(),
        ])
        .status()
        .await
        .context("Failed to spawn ffmpeg for concat")?;

    let _ = tokio::fs::remove_file(&list).await;
    if !status.success() {
        anyhow::bail!("Lossless concat failed");
    }
    if !out.exists() {
        anyhow::bail!("Concat produced no output");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Reviewer agent (LLM) → review scores
// ---------------------------------------------------------------------------

async fn run_reviewer(
    cfg: &PipelineConfig,
    payload: &NovaEditPayload,
    plan: &NovaEditPlan,
    shots: &[NovaShot],
) -> Result<NovaEditReview> {
    let brief = &payload.brief;
    let system_prompt = r#"You are the REVIEWER agent of an agentic video editor. Given the creative brief and the edit plan, score the resulting edit on five dimensions, each 0.0–1.0. `overall` is NOT an average; it is your editorial judgment of the composite. For short-form, judge hook strength, concise pacing, and payoff. For long-form, judge continuity, section structure, context preservation, sustained pacing, and conclusion quality; do not penalize necessary longer segments merely for not having short-form pacing. If overall < 0.7, feedback MUST be specific and actionable (reference segment positions, pacing, structure). Return STRICT JSON only:
{"adherence":0.0,"pacing":0.0,"visual_quality":0.0,"watchability":0.0,"overall":0.0,"feedback":"..."}"#;

    let mut entries = String::new();
    for e in &plan.entries {
        let shot = resolve_shot(shots, &e.shot_id);
        let transcript = shot.map(|s| s.transcript.trim()).unwrap_or("").to_string();
        entries.push_str(&format!(
            "- seg {}: shot `{}` [{:.2}s-{:.2}s] overlay={:?} | \"{}\"\n",
            e.position, e.shot_id, e.start_trim, e.end_trim, e.text_overlay, &transcript[..transcript.chars().take(90).count()]
        ));
    }

    let user_prompt = format!(
        "CREATIVE BRIEF:\n{}\n\nTARGET DURATION: {}s\nCUT DURATION: {:.1}s\n\nEDIT PLAN:\n{}\n\nReview this edit and return the JSON score object.",
        serde_json::to_string_pretty(brief)?,
        brief.duration_seconds,
        plan.total_duration,
        entries
    );

    let value = call_llm_json(cfg, payload, system_prompt, &user_prompt).await?;
    Ok(serde_json::from_value(value).unwrap_or(NovaEditReview {
        adherence: 0.5,
        pacing: 0.5,
        visual_quality: 0.5,
        watchability: 0.5,
        overall: 0.5,
        feedback: "Reviewer failed to produce structured output — treating as pass".into(),
    }))
}

// ---------------------------------------------------------------------------
// Generic LLM JSON call (Gemini or OpenRouter)
// ---------------------------------------------------------------------------

async fn call_llm_json(cfg: &PipelineConfig, payload: &NovaEditPayload, system_prompt: &str, user_prompt: &str) -> Result<Value> {
    let gemini_key = payload.api_keys.gemini_key.clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| (!cfg.gemini_api_key.trim().is_empty()).then(|| cfg.gemini_api_key.clone()))
        .unwrap_or_default();
    let openrouter_key = payload.api_keys.openrouter_key.clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok().filter(|s| !s.trim().is_empty()))
        .unwrap_or_default();

    let provider = if payload.llm_provider.is_empty() {
        DIRECTOR_DEFAULT_MODEL.to_string()
    } else {
        payload.llm_provider.clone()
    };
    let is_openrouter = provider.contains('/') || !openrouter_key.is_empty();

    if is_openrouter {
        let model = if provider.contains('/') { provider } else { "openrouter/free".to_string() };
        let url = "https://openrouter.ai/api/v1/chat/completions";
        let body = json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "temperature": 0.3
        });
        let client = reqwest::Client::new();
        let response = client.post(url)
            .header("Authorization", format!("Bearer {}", openrouter_key))
            .header("HTTP-Referer", "https://novaclip.app")
            .header("X-Title", "NovaClip")
            .json(&body)
            .send()
            .await
            .context("OpenRouter request failed")?;
        let resp: Value = response.json().await.context("Failed to parse OpenRouter response")?;
        let text = resp.pointer("/choices/0/message/content").and_then(|v| v.as_str()).unwrap_or("{}");
        let clean = strip_fences(text);
        Ok(serde_json::from_str(clean).unwrap_or_else(|_| Value::Null))
    } else {
        let model = if provider.contains('/') || provider.is_empty() { DIRECTOR_DEFAULT_MODEL.to_string() } else { provider };
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            model, gemini_key
        );
        let body = json!({
            "systemInstruction": {"parts": [{"text": system_prompt}]},
            "contents": [{"role": "user", "parts": [{"text": user_prompt}]}],
            "generationConfig": {"temperature": 0.3, "responseMimeType": "application/json"}
        });
        let client = reqwest::Client::new();
        let response = client.post(&url).json(&body).send().await.context("Gemini request failed")?;
        let resp: Value = response.json().await.context("Failed to parse Gemini response")?;
        if let Some(err) = resp.pointer("/error/message").and_then(|v| v.as_str()) {
            anyhow::bail!("Gemini API error: {}", err);
        }
        let text = resp.pointer("/candidates/0/content/parts/0/text").and_then(|v| v.as_str()).unwrap_or("{}");
        let clean = strip_fences(text);
        Ok(serde_json::from_str(clean).unwrap_or_else(|_| Value::Null))
    }
}

fn strip_fences(text: &str) -> &str {
    let t = text.trim();
    let t = t.strip_prefix("```json").unwrap_or(t);
    let t = t.strip_prefix("```").unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    t.trim()
}

// ---------------------------------------------------------------------------
// DB persistence helpers
// ---------------------------------------------------------------------------

async fn save_payload(db: &DbPool, task_id_str: &str, payload: &NovaEditPayload) -> Result<()> {
    sqlx::query("UPDATE tasks SET novaedit_payload = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(serde_json::to_string(payload)?)
        .bind(task_id_str)
        .execute(db)
        .await?;
    Ok(())
}

async fn save_edit_plan(db: &DbPool, task_id_str: &str, plan: &NovaEditPlan) -> Result<()> {
    sqlx::query("UPDATE tasks SET edit_plan = ?, updated_at = datetime('now') WHERE id = ?")
        .bind(serde_json::to_string(plan)?)
        .bind(task_id_str)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn read_edit_plan(db: &DbPool, task_id_str: &str) -> Result<NovaEditPlan> {
    let raw: Option<String> = sqlx::query_scalar("SELECT edit_plan FROM tasks WHERE id = ?")
        .bind(task_id_str)
        .fetch_one(db)
        .await?;
    match raw {
        Some(s) => serde_json::from_str::<NovaEditPlan>(&s)
            .map_err(|e| anyhow::anyhow!("Failed to parse edit plan: {}", e)),
        None => anyhow::bail!("No edit plan stored for task"),
    }
}

async fn finalize_clip(
    db: &DbPool,
    output_dir: &Path,
    final_path: &Path,
    task_id_str: &str,
    plan: &NovaEditPlan,
) -> Result<()> {
    let _ = output_dir;
    let clip_id = uuid::Uuid::new_v4().to_string();
    let total = plan.total_duration.max(0.1);

    sqlx::query(
        r#"INSERT INTO generated_clips
            (id, task_id, clip_order, filename, file_path, start_time, end_time, duration,
             transcript_text, relevance_score, virality_score, hook_score, engagement_score,
             value_score, shareability_score, hook_type, hook_title, reasoning)
           VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
    )
    .bind(&clip_id)
    .bind(task_id_str)
    .bind(0)
    .bind("final_video.mp4")
    .bind(final_path.to_str().unwrap_or_default())
    .bind("0:00")
    .bind(format!("0:{:02}", total as i32))
    .bind(total)
    .bind(&plan.rationale)
    .bind(1.0)
    .bind(90)
    .bind(90)
    .bind(90)
    .bind(90)
    .bind(90)
    .bind(Some("agentic".to_string()))
    .bind(Some("NovaEdit — agentic cut".to_string()))
    .bind(Some(&plan.rationale))
    .execute(db)
    .await?;

    info!("NovaEdit {} — final clip inserted: {}", task_id_str, clip_id);
    Ok(())
}
