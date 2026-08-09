pub mod pipeline;
pub use pipeline::PipelineConfig;

use novaclip_db::DbPool;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

pub use pipeline::tts::WordTimestamp;
pub use pipeline::studio_llm::ScriptItem;

struct WordRange { start_idx: usize, end_idx: usize, start_time: f64, end_time: f64 }

/// Start the in-process worker loop. Receives task IDs from the API via an
/// mpsc channel and processes each in a spawned Tokio task.
pub async fn run_worker(db: DbPool, mut rx: tokio::sync::mpsc::Receiver<Uuid>) {
    info!("NovaClip Worker started — waiting for in-process jobs...");
    while let Some(task_id) = rx.recv().await {
        let db_clone = db.clone();
        tokio::spawn(async move {
            info!("Processing task: {}", task_id);
            if let Err(e) = process_task(db_clone.clone(), task_id).await {
                error!("Pipeline failed for task {}: {}", task_id, e);
                sqlx::query(
                    "UPDATE tasks SET status = 'error', error_message = ?, updated_at = datetime('now') WHERE id = ?"
                )
                .bind(e.to_string())
                .bind(task_id.to_string())
                .execute(&db_clone)
                .await
                .ok();
            }
        });
    }
}

/// Write progress directly to the tasks table. Replaces Redis pub/sub.
pub async fn emit_progress(db: &DbPool, task_id: &str, percent: i32, message: &str, status: &str) {
    sqlx::query(
        "UPDATE tasks SET progress = ?, progress_message = ?, status = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(percent)
    .bind(message)
    .bind(status)
    .bind(task_id)
    .execute(db)
    .await
    .ok();
}

/// Returns true if cancel_requested is set; also updates status to 'cancelled'.
pub async fn check_cancelled(db: &DbPool, task_id: &str) -> bool {
    let flag: Option<i64> = sqlx::query_scalar(
        "SELECT cancel_requested FROM tasks WHERE id = ?"
    )
    .bind(task_id)
    .fetch_optional(db)
    .await
    .ok()
    .flatten();

    if flag.unwrap_or(0) != 0 {
        sqlx::query(
            "UPDATE tasks SET status = 'cancelled', updated_at = datetime('now') WHERE id = ?"
        )
        .bind(task_id)
        .execute(db)
        .await
        .ok();
        info!("Task {} cancelled by user", task_id);
        true
    } else {
        false
    }
}

async fn process_task(db: DbPool, task_id: Uuid) -> anyhow::Result<()> {
    let task_id_str = task_id.to_string();

    let task = match sqlx::query_as::<_, novaclip_db::Task>(
        "SELECT * FROM tasks WHERE id = ?"
    )
    .bind(&task_id_str)
    .fetch_optional(&db)
    .await
    {
        Ok(Some(t)) => t,
        Ok(None) => {
            warn!("Task {} not found in DB — skipping", task_id);
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    if task.cancel_requested {
        info!("Task {} was already cancelled — skipping", task_id);
        return Ok(());
    }

    let cfg = PipelineConfig {
        task_id,
        url: task.source_url.clone(),
        source_type: task.source_type.clone(),
        aspect_ratio: task.aspect_ratio.clone(),
        num_clips: task.num_clips,
        font_family: task.font_family.clone(),
        font_size: task.font_size,
        font_color: task.font_color.clone(),
        caption_template: task.caption_template.clone(),
        add_subtitles: task.add_subtitles,
        include_broll: task.include_broll,
        processing_mode: task.processing_mode.clone(),
        cut_long_pauses: task.cut_long_pauses,
        pause_threshold_ms: task.pause_threshold_ms,
        remove_filler_words: task.remove_filler_words,
        auto_vertical_reframe: task.auto_vertical_reframe,
        reframe_preset: task.reframe_preset.clone(),
        reframe_frame_skip: task.reframe_frame_skip.max(1) as u32,
        reframe_layout: task.reframe_layout.clone(),
        speaker_active_switch: task.speaker_active_switch,
        split_divider: task.split_divider,
        originality_boost: task.originality_boost.clone(),
        translate_language: task.translate_language.clone(),
        giphy_api_key: std::env::var("GIPHY_API_KEY").ok().filter(|s| !s.is_empty())
            .or_else(|| task.giphy_api_key.clone().filter(|s| !s.trim().is_empty())),
        filtered_words: serde_json::from_str::<Vec<String>>(&task.filtered_words)
            .unwrap_or_default(),
        output_dir: std::env::var("OUTPUT_DIR").unwrap_or_else(|_| "outputs".into()),
        temp_dir: std::env::var("TEMP_DIR").unwrap_or_else(|_| "temp".into()),
        gemini_api_key: task.gemini_api_key
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| std::env::var("GEMINI_API_KEY").unwrap_or_default()),
        gemini_model: std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-3.1-flash-lite".into()),
        llm_provider: task.llm_provider.clone(),
        deepgram_api_key: task.deepgram_api_key
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| std::env::var("DEEPGRAM_API_KEY").unwrap_or_default()),
        openrouter_api_key: task.openrouter_api_key
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| std::env::var("OPENROUTER_API_KEY").unwrap_or_default()),
        stt_provider: if task.stt_provider.trim().is_empty() {
            std::env::var("STT_PROVIDER").unwrap_or_else(|_| "deepgram".into())
        } else {
            task.stt_provider.clone()
        },
        vosk_model_path: std::env::var("VOSK_MODEL_PATH")
            .unwrap_or_else(|_| "models/vosk-model-small-en-us-0.15".into()),
        whisper_model_path: std::env::var("WHISPER_MODEL_PATH")
            .unwrap_or_else(|_| "models/ggml-base.bin".into()),
        pyannote_segmentation_model_path: std::env::var("PYANNOTE_SEGMENTATION_MODEL_PATH")
            .unwrap_or_else(|_| "models/segmentation-3.0.onnx".into()),
        pyannote_embedding_model_path: std::env::var("PYANNOTE_EMBEDDING_MODEL_PATH")
            .unwrap_or_else(|_| "models/wespeaker_en_voxceleb_CAM++.onnx".into()),
        pexels_api_key: std::env::var("PEXELS_API_KEY").ok().filter(|s| !s.is_empty()),
        pixabay_api_key: std::env::var("PIXABAY_API_KEY").ok().filter(|s| !s.is_empty()),
        studio_payload: task.studio_payload.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        highlight_color: task.highlight_color.clone(),
        caption_animation: task.caption_animation.clone(),
        auto_emojis: task.auto_emojis,
        watermark_position: task.watermark_position.clone(),
        watermark_opacity: task.watermark_opacity,
        watermark_path: task.watermark_path.clone(),
        novaedit_payload: task.novaedit_payload.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        edit_plan: task.edit_plan.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        review_score: task.review_score.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        repurpose_payload: task.repurpose_payload.as_ref().and_then(|s| serde_json::from_str(s).ok()),
        repurpose_result: task.repurpose_result.as_ref().and_then(|s| serde_json::from_str(s).ok()),
    };

    let output_dir = PathBuf::from(&cfg.output_dir).join(task_id.to_string());
    tokio::fs::create_dir_all(&output_dir).await?;

    if cfg.source_type == "agentic" {
        // NovaEdit fully manages its own status transitions (awaiting_approval / completed)
        return pipeline::nova_edit::process_nova_edit_task(&db, &cfg, &output_dir, &task_id_str).await;
    }

    if cfg.source_type == "repurpose" {
        return pipeline::repurpose::process_repurpose_task(&db, &cfg, &output_dir, &task_id_str).await;
    }

    if cfg.source_type == "studio" {
        process_studio_task(&db, &cfg, &output_dir, &task_id_str).await?;
        publish_studio_video_if_requested(&db, &cfg, &output_dir, &task_id_str).await;
    } else {
        process_standard_task(&db, &cfg, &output_dir, &task_id_str).await?;
    }

    sqlx::query(
        "UPDATE tasks SET status = 'completed', progress = 100, progress_message = 'All clips ready!', completed_at = datetime('now'), updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&task_id_str)
    .execute(&db)
    .await?;

    info!("Task {} completed", task_id);
    Ok(())
}

/// After a studio task finishes, publish the final video to YouTube via
/// Upload-Post when the AI Shorts payload requests it (shorts_payload.publish).
async fn publish_studio_video_if_requested(
    db: &DbPool,
    cfg: &PipelineConfig,
    output_dir: &Path,
    task_id_str: &str,
) {
    let Some(payload) = cfg.studio_payload.as_ref() else { return };
    if payload.get("mode").and_then(|v| v.as_str()) != Some("ai-shorts") {
        return;
    }
    let shorts = payload.get("shorts_payload").cloned().unwrap_or_else(|| serde_json::json!({}));
    if shorts.get("publish").and_then(|v| v.as_bool()).unwrap_or(false) != true {
        return;
    }
    let Some(api_key) = payload
        .pointer("/api_keys/uploadpost_key")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
    else {
        warn!("[Publish] AI Shorts auto-publish requested but no Upload-Post API key was provided");
        return;
    };

    let video_path = output_dir.join("final_video.mp4");
    if !video_path.exists() {
        warn!("[Publish] Final video missing at {} — skipping auto-publish", video_path.display());
        return;
    }

    let script = payload.get("script").and_then(|v| v.as_str()).unwrap_or("");
    let product_desc = shorts.get("product_description").and_then(|v| v.as_str()).unwrap_or("");
    let title = product_desc.split('\n').next().unwrap_or(script).trim();
    let title = if title.is_empty() { "AI Short".to_string() } else { title.chars().take(70).collect() };
    let description = format!("{}\n\n#shorts #ai #product", script.trim());

    let profile = shorts
        .get("uploadpost_profile")
        .and_then(|v| v.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string());

    let username = match profile {
        Some(p) => p,
        None => match pipeline::uploadpost::resolve_profile(api_key).await {
            Ok(u) => u,
            Err(e) => {
                warn!("[Publish] Failed to resolve Upload-Post profile: {}", e);
                return;
            }
        },
    };

    emit_progress(db, task_id_str, 96, "Publishing to YouTube (Upload-Post)...", "processing").await;
    info!("[Publish] Uploading {} to YouTube via Upload-Post as '{}'", video_path.display(), username);
    match pipeline::uploadpost::publish_video(api_key, &username, &video_path, &title, &description, &["youtube".to_string()]).await {
        Ok(body) => {
            emit_progress(db, task_id_str, 99, "Published to YouTube!", "processing").await;
            info!("[Publish] YouTube publish submitted successfully: {}", body);
        }
        Err(e) => {
            warn!("[Publish] Upload-Post publish failed: {}", e);
        }
    }
}

/// Standard pipeline: download video, transcribe, analyze, extract clips
async fn process_standard_task(
    db: &DbPool,
    cfg: &PipelineConfig,
    output_dir: &Path,
    task_id_str: &str,
) -> anyhow::Result<()> {
    use pipeline::{
        analyze::analyze_transcript,
        caption::{burn_captions, get_caption_style, get_clip_words},
        clip::{build_crop_filter, extract_clip},
        crop::output_dimensions,
        reframe::{apply_vertical_reframe, ReframeOptions},
        originality::apply_originality_boost,
        translate::translate_words,
        dedup::{dedup_segments, timestamp_to_seconds},
        download::{download_youtube, extract_audio, get_video_duration, resolve_upload_path},
        transcribe::{
            build_transcript_for_prompt, speaker_segments_for_window, transcribe_audio,
        },
    };

    let task_id = cfg.task_id;

    emit_progress(db, task_id_str, 5, "Downloading video...", "processing").await;

    let video_path = if cfg.source_type == "upload" {
        let p = resolve_upload_path(&cfg.url, &cfg.temp_dir);
        if !p.exists() {
            anyhow::bail!("Uploaded file not found: {}", p.display());
        }
        p
    } else {
        download_youtube(&cfg.url, &cfg.temp_dir).await?
    };

    if check_cancelled(db, task_id_str).await {
        return Ok(());
    }

    let duration = get_video_duration(&video_path).await?;
    info!("Video duration: {:.1}s", duration);

    emit_progress(db, task_id_str, 15, "Extracting audio...", "processing").await;
    let audio_path = extract_audio(&video_path, &cfg.temp_dir).await?;

    emit_progress(db, task_id_str, 25, "Transcribing audio...", "processing").await;
    let transcript = transcribe_audio(
        &audio_path,
        &cfg.stt_provider,
        &cfg.deepgram_api_key,
        Path::new(&cfg.vosk_model_path),
        Path::new(&cfg.whisper_model_path),
        Path::new(&cfg.pyannote_segmentation_model_path),
        Path::new(&cfg.pyannote_embedding_model_path),
    ).await?;
    let transcript_text = build_transcript_for_prompt(&transcript);

    sqlx::query(
        "UPDATE tasks SET transcript_cache = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&transcript_text)
    .bind(task_id_str)
    .execute(db)
    .await
    .ok();

    if check_cancelled(db, task_id_str).await {
        return Ok(());
    }

    emit_progress(db, task_id_str, 40, "Analyzing virality with AI...", "processing").await;
    let analysis = analyze_transcript(
        &transcript_text,
        cfg.num_clips,
        &cfg.llm_provider,
        &cfg.gemini_api_key,
        &cfg.openrouter_api_key,
    )
    .await?;

    let deduped = dedup_segments(analysis.most_relevant_segments, cfg.num_clips as usize);
    info!("Selected {} clips after dedup", deduped.len());

    if check_cancelled(db, task_id_str).await {
        return Ok(());
    }

    let (out_width, out_height) = output_dimensions(&cfg.aspect_ratio);
    let mut caption_style = get_caption_style(
        &cfg.caption_template,
        &cfg.font_family,
        cfg.font_size,
        &cfg.font_color,
    );
    {
        let h = cfg.highlight_color.trim_start_matches('#');
        if h.len() == 6 {
            caption_style.highlight_color = format!("&H00{}{}{}&", &h[4..6], &h[2..4], &h[0..2]);
        }
    }
    caption_style.word_pop = cfg.caption_animation == "word_pop";
    let total_clips = deduped.len();

    let use_reframe = cfg.auto_vertical_reframe && cfg.aspect_ratio == "9:16";

    for (i, seg) in deduped.iter().enumerate() {
        if check_cancelled(db, task_id_str).await {
            return Ok(());
        }

        let pct = 50 + (i * 45 / total_clips.max(1)) as i32;
        let msg = format!("Rendering clip {} of {}...", i + 1, total_clips);
        emit_progress(db, task_id_str, pct, &msg, "processing").await;

        let start_secs = timestamp_to_seconds(&seg.start_time);
        let end_secs = timestamp_to_seconds(&seg.end_time);

        let raw_clip_path = extract_clip(
            &video_path,
            output_dir,
            i as i32,
            &seg.start_time,
            &seg.end_time,
            &cfg.aspect_ratio,
            &task_id.to_string(),
            use_reframe,
        )
        .await?;

        let reframed_path = if use_reframe {
            let reframe_output = output_dir.join(format!("reframe_{}_{}.mp4", i + 1, &task_id_str[..8]));
            let speaker_json_path = if cfg.reframe_layout != "single" && cfg.speaker_active_switch {
                let segments = speaker_segments_for_window(&transcript.words, start_secs, end_secs);
                if segments.is_empty() {
                    None
                } else {
                    let p = output_dir.join(format!("speaker_{}_{}.json", i + 1, &task_id_str[..8]));
                    tokio::fs::write(&p, serde_json::to_vec(&segments).unwrap_or_default()).await.ok();
                    Some(p)
                }
            } else {
                None
            };
            let reframe_opts = ReframeOptions {
                layout: &cfg.reframe_layout,
                speaker_aware: cfg.speaker_active_switch,
                split_divider: cfg.split_divider,
                speaker_json: speaker_json_path.as_deref(),
            };
            let reframe_result = apply_vertical_reframe(
                &raw_clip_path,
                &reframe_output,
                &cfg.reframe_preset,
                cfg.reframe_frame_skip,
                &reframe_opts,
            )
            .await;
            if let Some(p) = &speaker_json_path {
                tokio::fs::remove_file(p).await.ok();
            }
            match reframe_result {
                Ok(()) => {
                    tokio::fs::remove_file(&raw_clip_path).await.ok();
                    reframe_output
                }
                Err(e) => {
                    warn!("Vertical reframe failed: {} — falling back to center crop", e);
                    let fallback = output_dir.join(format!("fallback_{}_{}.mp4", i + 1, &task_id_str[..8]));
                    let crop_filter = build_crop_filter(&cfg.aspect_ratio);
                    if crop_filter.is_empty() {
                        raw_clip_path
                    } else {
                        let status = tokio::process::Command::new("ffmpeg")
                            .args([
                                "-y", "-i",
                                raw_clip_path.to_str().unwrap(),
                                "-vf", &crop_filter,
                                "-c:v", "libx264", "-preset", "fast", "-crf", "18",
                                "-pix_fmt", "yuv420p",
                                "-c:a", "copy",
                                fallback.to_str().unwrap(),
                            ])
                            .status().await;
                        match status {
                            Ok(s) if s.success() => {
                                tokio::fs::remove_file(&raw_clip_path).await.ok();
                                fallback
                            }
                            _ => {
                                warn!("Fallback crop also failed — using raw clip");
                                raw_clip_path
                            }
                        }
                    }
                }
            }
        } else {
            raw_clip_path
        };

        let clip_for_captions = reframed_path;
        let clip_word_refs = get_clip_words(&transcript.words, start_secs, end_secs);
        let mut caption_owned: Vec<crate::pipeline::transcribe::DeepgramWord> = clip_word_refs.iter().map(|w| (*w).clone()).collect();

        if !cfg.translate_language.is_empty() && cfg.add_subtitles {
            match translate_words(&clip_word_refs, &cfg.translate_language, &cfg.gemini_model, &cfg.gemini_api_key).await {
                Ok(translated) => caption_owned = translated,
                Err(e) => warn!("Translation failed: {} — using original", e),
            }
        }
        if cfg.auto_emojis {
            let txt = &seg.text;
            if let Some(emoji) = get_sentence_emoji(txt) {
                if let Some(last) = caption_owned.last_mut() {
                    last.word.push(' ');
                    last.word.push_str(emoji);
                    last.punctuated_word = Some(last.word.clone());
                }
            }
        }
        let caption_words: Vec<&crate::pipeline::transcribe::DeepgramWord> = caption_owned.iter().collect();

        let final_path = if cfg.add_subtitles {
            match burn_captions(
                &clip_for_captions,
                output_dir,
                &caption_words,
                start_secs,
                &caption_style,
                seg.hook_title.as_deref(),
                end_secs - start_secs,
                out_width,
                out_height,
            )
            .await
            {
                Ok(p) => {
                    tokio::fs::remove_file(&clip_for_captions).await.ok();
                    p
                }
                Err(e) => {
                    warn!("Caption burn failed: {} — using raw clip", e);
                    clip_for_captions
                }
            }
        } else {
            clip_for_captions
        };

        let boosted_path = if cfg.originality_boost != "none" {
            let boost_output = output_dir.join(format!("boost_{}_{}.mp4", i + 1, &task_id_str[..8]));
            match apply_originality_boost(&final_path, &boost_output, &cfg.originality_boost).await {
                Ok(()) => {
                    tokio::fs::remove_file(&final_path).await.ok();
                    boost_output
                }
                Err(e) => {
                    warn!("Originality boost failed: {} — skipping", e);
                    final_path
                }
            }
        } else {
            final_path
        };

        let filename = boosted_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let duration_secs =
            timestamp_to_seconds(&seg.end_time) - timestamp_to_seconds(&seg.start_time);
        let clip_id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"INSERT INTO generated_clips
                (id, task_id, clip_order, filename, file_path, start_time, end_time, duration,
                 transcript_text, relevance_score, virality_score, hook_score, engagement_score,
                 value_score, shareability_score, hook_type, hook_title, reasoning)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
        )
        .bind(&clip_id)
        .bind(task_id_str)
        .bind(i as i32)
        .bind(&filename)
        .bind(boosted_path.to_str().unwrap())
        .bind(&seg.start_time)
        .bind(&seg.end_time)
        .bind(duration_secs)
        .bind(&seg.text)
        .bind(seg.relevance_score)
        .bind(seg.virality.total_score)
        .bind(seg.virality.hook_score)
        .bind(seg.virality.engagement_score)
        .bind(seg.virality.value_score)
        .bind(seg.virality.shareability_score)
        .bind(&seg.virality.hook_type)
        .bind(seg.hook_title.as_deref())
        .bind(&seg.reasoning)
        .execute(db)
        .await?;
    }

    tokio::fs::remove_file(&audio_path).await.ok();
    info!("Standard task {} completed — {} clips generated", task_id, total_clips);
    Ok(())
}

/// Studio faceless AI pipeline: decompose script, scrape media, full voiceover, global captions
async fn process_studio_task(
    db: &DbPool,
    cfg: &PipelineConfig,
    output_dir: &Path,
    task_id_str: &str,
) -> anyhow::Result<()> {
    use pipeline::{
        studio_llm::StudioLlmProcessor,
        tts::TtsEngine,
        scraper::StudioScraper,
        caption::get_caption_style,
        crop::output_dimensions,
    };

    let payload = cfg.studio_payload.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No studio_payload found for studio task"))?;

    let script = payload["script"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing script in studio_payload"))?;
    let mode = payload["mode"].as_str().unwrap_or("stock");
    let llm_provider = payload["llm_provider"].as_str().unwrap_or("gemini-3.1-flash-lite");
    let mut tts_provider = payload["tts_provider"].as_str().unwrap_or("edge-tts");
    if mode == "ai" && !matches!(tts_provider, "elevenlabs" | "deepgram-aura") {
        warn!("AI B-Roll mode requires Deepgram or ElevenLabs TTS (word timestamps); using Deepgram Aura instead of '{}'", tts_provider);
        tts_provider = "deepgram-aura";
    }
    let voice = payload["voice"].as_str().unwrap_or("en-US-ChristopherNeural");
    let target_duration = payload["duration"].as_i64().unwrap_or(60) as i32;
    let scraper_source = payload["source"].as_str().unwrap_or("all");
    let media_type = payload["media_type"].as_str().unwrap_or("video");
    let vibe = payload["vibe"].as_str().unwrap_or("aesthetic");
    let subtitle_style = payload["subtitle_style"].as_str().unwrap_or("high_retention");
    let bg_music = payload["bg_music"].as_str().unwrap_or("none");

    let gemini_key = payload["api_keys"]["gemini_key"].as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| Some(&cfg.gemini_api_key))
        .unwrap_or("");
    let deepgram_key = payload["api_keys"]["deepgram_key"].as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| Some(&cfg.deepgram_api_key))
        .unwrap_or("");
    let openrouter_key = payload["api_keys"]["openrouter_key"].as_str().unwrap_or("");
    let elevenlabs_key = payload["api_keys"]["elevenlabs_key"].as_str().unwrap_or("");
    let pexels_key = payload["api_keys"]["pexels_key"].as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| cfg.pexels_api_key.as_deref())
        .unwrap_or("");
    let pixabay_key = payload["api_keys"]["pixabay_key"].as_str()
        .filter(|s| !s.is_empty())
        .or_else(|| cfg.pixabay_api_key.as_deref())
        .unwrap_or("");
    let wavespeed_key = payload["api_keys"]["wavespeed_key"].as_str()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| std::env::var("WAVESPEED_API_KEY").ok().filter(|s| !s.is_empty()))
        .unwrap_or_default();

    if (mode == "ai" || mode == "ai-shorts") && wavespeed_key.is_empty() {
        anyhow::bail!("{} mode requires a WaveSpeed API key. Add it in Settings.", if mode == "ai" { "AI B-Roll" } else { "AI Shorts" });
    }

    let (out_width, out_height) = output_dimensions(&cfg.aspect_ratio);

    let caption_template = match subtitle_style {
        "yellow_box" => "vibrant",
        "bold_outline" => "bold",
        "minimal" => "minimal",
        _ => "default",
    };
    let mut caption_style = get_caption_style(
        caption_template,
        &cfg.font_family,
        cfg.font_size,
        &cfg.font_color,
    );
    {
        let h = cfg.highlight_color.trim_start_matches('#');
        if h.len() == 6 {
            caption_style.highlight_color = format!("&H00{}{}{}&", &h[4..6], &h[2..4], &h[0..2]);
        }
    }
    caption_style.word_pop = cfg.caption_animation == "word_pop";

    // Step 1: Decompose script into sentence + keyword items
    emit_progress(db, task_id_str, 10, "Decomposing script with AI...", "processing").await;
    let processor = StudioLlmProcessor::new(
        gemini_key.to_string(),
        llm_provider.to_string(),
        openrouter_key.to_string(),
    );
    let items = processor.decompose_script(script, target_duration).await
        .map_err(|e| anyhow::anyhow!("Script decomposition failed: {}", e))?;

    if items.is_empty() {
        anyhow::bail!("Script decomposition returned zero items");
    }

    let total_items = items.len();
    info!("Decomposed script into {} items", total_items);

    let tts = TtsEngine::new(
        PathBuf::from(&cfg.temp_dir),
        elevenlabs_key.to_string(),
        deepgram_key.to_string(),
    );
    let scraper = StudioScraper::new(
        PathBuf::from(&cfg.temp_dir).join("studio_media"),
        pexels_key.to_string(),
        pixabay_key.to_string(),
    );

    // Step 2: Generate full-script voiceover + word timestamps
    emit_progress(db, task_id_str, 15, "Generating full voiceover...", "processing").await;
    let full_script_text: Vec<&str> = items.iter().map(|i| i.sentence.as_str()).collect();
    let full_script_joined = full_script_text.join(" ");

    let (full_audio_path, word_timestamps) = tts.synthesize_full(&full_script_joined, tts_provider, voice)
        .await
        .map_err(|e| anyhow::anyhow!("Full script TTS failed: {}", e))?;

    let mut temp_files: Vec<PathBuf> = vec![full_audio_path.clone()];
    let total_audio_dur = word_timestamps.last().map(|w| w.end).unwrap_or(10.0);
    info!("Full voiceover generated: {} words, {:.1}s", word_timestamps.len(), total_audio_dur);

    // Map each sentence item to a range of word indices in the timestamp list
    let mut sentence_word_ranges: Vec<WordRange> = Vec::new();
    let mut char_offset = 0;
    let mut word_idx = 0;
    for item in &items {
        let sentence_lower = item.sentence.to_lowercase();
        let start_idx = word_idx;
        let mut matched_word_count = 0;
        let sentence_words: Vec<&str> = sentence_lower.split_whitespace().collect();
        while word_idx < word_timestamps.len() && matched_word_count < sentence_words.len() {
            let tw = word_timestamps[word_idx].word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
            let sw = sentence_words[matched_word_count].trim_matches(|c: char| !c.is_alphanumeric());
            if tw == sw || tw.is_empty() {
                matched_word_count += 1;
            } else if matched_word_count > 0 {
                let mut check_ahead = false;
                for look in 1..=3 {
                    if word_idx + look < word_timestamps.len() {
                        let nw = word_timestamps[word_idx + look].word.to_lowercase().trim_matches(|c: char| !c.is_alphanumeric()).to_string();
                        if nw == *sw {
                            check_ahead = true;
                            break;
                        }
                    }
                }
                if !check_ahead {
                    matched_word_count = 0;
                }
            }
            char_offset += word_timestamps[word_idx].word.len() + 1;
            word_idx += 1;
        }
        let end_idx = word_idx.min(word_timestamps.len());
        let st = if start_idx < word_timestamps.len() { word_timestamps[start_idx].start } else { total_audio_dur };
        let et = if end_idx > 0 && end_idx <= word_timestamps.len() { word_timestamps[end_idx - 1].end } else { total_audio_dur };

        if start_idx == end_idx {
            let frac = char_offset as f64 / full_script_joined.len() as f64;
            let est_start = frac * total_audio_dur;
            let est_end = (char_offset + item.sentence.len()) as f64 / full_script_joined.len() as f64 * total_audio_dur;
            sentence_word_ranges.push(WordRange { start_idx, end_idx, start_time: est_start, end_time: est_end });
        } else {
            sentence_word_ranges.push(WordRange { start_idx, end_idx, start_time: st, end_time: et });
        }
    }

    // Step 3: Fetch ALL media clips first (one per item), store paths.
    // Stock mode scrapes Pexels/Pixabay; AI mode generates Seedance clips.
    emit_progress(db, task_id_str, 20, "Fetching media clips...", "processing").await;
    let mut media_paths: Vec<PathBuf> = Vec::new();

    if mode == "ai" {
        // AI B-Roll: detailed visual prompts -> WaveSpeed Seedance clips (Pexels fallback)
        let sentences: Vec<String> = items.iter().map(|i| i.sentence.clone()).collect();
        let keywords: Vec<String> = items.iter().map(|i| i.keyword.clone()).collect();
        emit_progress(db, task_id_str, 20, "Writing AI B-roll prompts...", "processing").await;
        let prompts = processor
            .generate_ai_clip_prompts(&full_script_joined, &sentences, &keywords.first().cloned().unwrap_or_default())
            .await;
        let clip_sec = std::env::var("WAVESPEED_VIDEO_DURATION")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(5);
        let ai_media_dir = output_dir.join("ai_media");
        emit_progress(db, task_id_str, 22, "Generating AI B-roll clips (Seedance)...", "processing").await;
        let ai_paths = pipeline::wavespeed::generate_ai_media_paths(
            wavespeed_key.clone(),
            prompts.clone(),
            cfg.aspect_ratio.clone(),
            clip_sec,
            ai_media_dir,
            pexels_key.to_string(),
            keywords.clone(),
        )
        .await;
        for (i, p) in ai_paths.iter().enumerate() {
            if let Some(p) = p {
                temp_files.push(p.clone());
                media_paths.push(p.clone());
            } else {
                warn!("AI clip {} could not be generated — using placeholder", i + 1);
                let placeholder = output_dir.join(format!("placeholder_{:02}.mp4", i + 1));
                let blank_dur = if i < sentence_word_ranges.len() {
                    (sentence_word_ranges[i].end_time - sentence_word_ranges[i].start_time).max(2.0)
                } else {
                    (total_audio_dur / total_items as f64).max(2.0)
                };
                let dur_str = format!("{:.3}", blank_dur);
                let vf = format!("color=c=black:s={}x{}:d={}", out_width, out_height, dur_str);
                let status = Command::new("ffmpeg")
                    .args(["-y", "-f", "lavfi", "-i", &vf])
                    .args(["-c:v", "libx264", "-preset", "fast", "-crf", "23", "-pix_fmt", "yuv420p"])
                    .arg(placeholder.to_str().unwrap())
                    .status().await;
                match status {
                    Ok(s) if s.success() => {
                        temp_files.push(placeholder.clone());
                        media_paths.push(placeholder);
                    }
                    _ => {
                        warn!("Placeholder creation failed for scene {} — skipping entirely", i + 1);
                        continue;
                    }
                }
            }
        }
        if media_paths.is_empty() {
            anyhow::bail!("All AI clip generation attempts failed — check your WaveSpeed key and model quota");
        }
        let actual_total = media_paths.len().min(items.len());
        // BGM: Lyria (AI-generated) when requested
        let full_bgm = if bg_music != "none" {
            match pipeline::wavespeed::generate_background_music(&wavespeed_key, &full_script_joined, &output_dir).await {
                Ok(p) => {
                    temp_files.push(p.clone());
                    Some(p)
                }
                Err(e) => {
                    warn!("Lyria BGM generation failed: {} — continuing without BGM", e);
                    None
                }
            }
        } else {
            None
        };
        let mixed_full_audio = if let Some(ref bgm_path) = full_bgm {
            let mixed = output_dir.join("mixed_full_audio.mp3");
            match mix_audio(&full_audio_path, bgm_path, &mixed).await {
                Ok(()) => {
                    temp_files.push(mixed.clone());
                    mixed
                }
                Err(e) => {
                    warn!("Audio mixing failed: {} — using raw TTS", e);
                    full_audio_path.clone()
                }
            }
        } else {
            full_audio_path.clone()
        };
        // Re-run the common assembly/render path with the AI media
        return finish_studio_render(
            db, cfg, output_dir, task_id_str,
            items.as_slice(),
            &word_timestamps,
            &sentence_word_ranges,
            &media_paths,
            actual_total,
            &mixed_full_audio,
            &caption_style,
            out_width, out_height,
            total_audio_dur,
            vibe,
        )
        .await
        .map(|_| ());
    } else if mode == "ai-shorts" {
        // AI Shorts: Flux 2 Pro actor portrait → lip-synced talking head →
        // Seedance B-roll → composite. The talking head anchors the hook scene
        // (scene 0), B-roll covers the remaining sentences.
        let shorts = payload.get("shorts_payload").cloned().unwrap_or_else(|| serde_json::json!({}));
        let cost_mode = shorts["cost_mode"].as_str().unwrap_or("low");
        let actor_description = shorts["actor_description"].as_str().unwrap_or("");
        let product_description = shorts["product_description"].as_str().unwrap_or("");
        let actor_audio_url = shorts["actor_audio_url"].as_str()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.to_string())
            .or_else(|| std::env::var("WAVESPEED_AUDIO_URL").ok().filter(|s| !s.trim().is_empty()));
        let audio_url = actor_audio_url.as_deref();

        let portrait_prompt = if !actor_description.trim().is_empty() {
            format!(
                "{}. Front-facing waist-up portrait, eyes on camera, mouth relaxed and visible, photorealistic, professional studio lighting, clean blurred background, 9:16 vertical.",
                actor_description.trim()
            )
        } else if !product_description.trim().is_empty() {
            format!(
                "Professional UGC creator portrait, front-facing waist-up, eyes on camera, mouth relaxed and visible, promoting: {}. Photorealistic, studio lighting, clean blurred background, 9:16 vertical.",
                product_description.trim()
            )
        } else {
            "Professional UGC creator portrait, front-facing waist-up, eyes on camera, mouth relaxed and visible, friendly confident expression, photorealistic, studio lighting, clean blurred background, 9:16 vertical.".to_string()
        };

        let hook_text = items.first().map(|i| i.sentence.clone()).unwrap_or_else(|| full_script_joined.clone());
        let hook_dur = sentence_word_ranges.first()
            .map(|r| (r.end_time - r.start_time).clamp(5.0, 15.0))
            .unwrap_or(8.0) as i32;

        // Premium InfiniteTalk lip-sync needs a hosted audio URL. If the caller
        // didn't provide one, synthesize the hook audio locally with the same TTS
        // voice and host it temporarily so the actor's lips match the voiceover.
        let hook_audio_path = if cost_mode == "premium" && audio_url.is_none() {
            match tts.synthesize(&hook_text, 0, tts_provider, voice).await {
                Ok(p) => {
                    temp_files.push(p.clone());
                    Some(p)
                }
                Err(e) => {
                    warn!("Hook audio TTS failed ({}): premium talking head will use AI Talking Photos text sync", e);
                    None
                }
            }
        } else {
            None
        };

        emit_progress(db, task_id_str, 30, "Generating AI actor portrait (Flux 2 Pro)...", "processing").await;
        let shorts_media_dir = output_dir.join("shorts_media");
        let actor = pipeline::wavespeed::generate_ai_actor(
            &wavespeed_key,
            &portrait_prompt,
            &hook_text,
            hook_dur,
            audio_url,
            hook_audio_path.as_deref(),
            cost_mode == "premium",
            &shorts_media_dir,
        )
        .await;

        let mut media_paths: Vec<PathBuf> = Vec::new();
        match actor {
            Ok((_portrait, talking)) => {
                info!("AI actor talking head generated: {:?}", talking);
                media_paths.push(talking.clone());
                temp_files.push(talking);
            }
            Err(e) => {
                warn!("AI actor generation failed ({}): {} — continuing with B-roll only", cost_mode, e);
            }
        }

        // B-roll: Seedance clips per sentence (Pexels fallback), same as AI mode
        let sentences: Vec<String> = items.iter().map(|i| i.sentence.clone()).collect();
        let keywords: Vec<String> = items.iter().map(|i| i.keyword.clone()).collect();
        emit_progress(db, task_id_str, 45, "Writing B-roll prompts...", "processing").await;
        let prompts = processor
            .generate_ai_clip_prompts(&full_script_joined, &sentences, &keywords.first().cloned().unwrap_or_default())
            .await;
        let clip_sec = std::env::var("WAVESPEED_VIDEO_DURATION")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(5);
        let ai_media_dir = output_dir.join("ai_media");
        emit_progress(db, task_id_str, 50, "Generating B-roll clips (Seedance)...", "processing").await;
        let ai_paths = pipeline::wavespeed::generate_ai_media_paths(
            wavespeed_key.clone(),
            prompts.clone(),
            cfg.aspect_ratio.clone(),
            clip_sec,
            ai_media_dir,
            pexels_key.to_string(),
            keywords.clone(),
        )
        .await;
        for (i, p) in ai_paths.iter().enumerate() {
            if let Some(p) = p {
                temp_files.push(p.clone());
                media_paths.push(p.clone());
            } else {
                warn!("B-roll clip {} could not be generated — skipping", i + 1);
            }
        }

        if media_paths.is_empty() {
            anyhow::bail!("AI Shorts: all media generation attempts failed — check your WaveSpeed key and model quota");
        }
        let actual_total = media_paths.len().min(items.len());

        // BGM: Lyria (AI-generated) when requested
        let full_bgm = if bg_music != "none" {
            match pipeline::wavespeed::generate_background_music(&wavespeed_key, &full_script_joined, &output_dir).await {
                Ok(p) => {
                    temp_files.push(p.clone());
                    Some(p)
                }
                Err(e) => {
                    warn!("Lyria BGM generation failed: {} — continuing without BGM", e);
                    None
                }
            }
        } else {
            None
        };
        let mixed_full_audio = if let Some(ref bgm_path) = full_bgm {
            let mixed = output_dir.join("mixed_full_audio.mp3");
            match mix_audio(&full_audio_path, bgm_path, &mixed).await {
                Ok(()) => {
                    temp_files.push(mixed.clone());
                    mixed
                }
                Err(e) => {
                    warn!("Audio mixing failed: {} — using raw TTS", e);
                    full_audio_path.clone()
                }
            }
        } else {
            full_audio_path.clone()
        };

        return finish_studio_render(
            db, cfg, output_dir, task_id_str,
            items.as_slice(),
            &word_timestamps,
            &sentence_word_ranges,
            &media_paths,
            actual_total,
            &mixed_full_audio,
            &caption_style,
            out_width, out_height,
            total_audio_dur,
            vibe,
        )
        .await
        .map(|_| ());
    }

    for (i, item) in items.iter().enumerate() {
        if check_cancelled(db, task_id_str).await {
            return Ok(());
        }
        let msg = format!("Fetching media {} of {}...", i + 1, total_items);
        emit_progress(db, task_id_str, 20 + (i * 20 / total_items.max(1)) as i32, &msg, "processing").await;
        match scraper.fetch_media(&item.keyword, scraper_source, media_type).await {
            Ok(p) => {
                temp_files.push(p.clone());
                media_paths.push(p);
            }
            Err(e) => {
                warn!("Media scrape failed for scene {}: {} — using placeholder", i + 1, e);
                // Create a blank placeholder clip
                let placeholder = output_dir.join(format!("placeholder_{:02}.mp4", i + 1));
                let blank_dur = if i < sentence_word_ranges.len() {
                    (sentence_word_ranges[i].end_time - sentence_word_ranges[i].start_time).max(2.0)
                } else {
                    (total_audio_dur / total_items as f64).max(2.0)
                };
                let dur_str = format!("{:.3}", blank_dur);
                let vf = format!("color=c=black:s={}x{}:d={}", out_width, out_height, dur_str);
                let status = Command::new("ffmpeg")
                    .args(["-y", "-f", "lavfi", "-i", &vf])
                    .args(["-c:v", "libx264", "-preset", "fast", "-crf", "23", "-pix_fmt", "yuv420p"])
                    .arg(placeholder.to_str().unwrap())
                    .status().await;
                match status {
                    Ok(s) if s.success() => {
                        temp_files.push(placeholder.clone());
                        media_paths.push(placeholder);
                    }
                    _ => {
                        warn!("Placeholder creation failed for scene {} — skipping entirely", i + 1);
                        continue;
                    }
                }
            }
        }
    }

    if media_paths.is_empty() {
        anyhow::bail!("All media fetch attempts failed");
    }
    let actual_total = media_paths.len().min(items.len());

    // Step 4: Generate BGM and mix with full voiceover
    let full_bgm = if bg_music != "none" {
        let bgm_path = output_dir.join("full_bgm.mp3");
        match generate_bgm(&bgm_path, bg_music, total_audio_dur).await {
            Ok(()) => {
                temp_files.push(bgm_path.clone());
                Some(bgm_path)
            }
            Err(e) => {
                warn!("BGM generation failed: {} — continuing without BGM", e);
                None
            }
        }
    } else {
        None
    };

    let mixed_full_audio = if let Some(ref bgm_path) = full_bgm {
        let mixed = output_dir.join("mixed_full_audio.mp3");
        match mix_audio(&full_audio_path, bgm_path, &mixed).await {
            Ok(()) => {
                temp_files.push(mixed.clone());
                mixed
            }
            Err(e) => {
                warn!("Audio mixing failed: {} — using raw TTS", e);
                full_audio_path.clone()
            }
        }
    } else {
        full_audio_path.clone()
    };

    // Step 5-7: Trim scenes, concat, captions, final render, insert clip record
    finish_studio_render(
        db, cfg, output_dir, task_id_str,
        items.as_slice(),
        &word_timestamps,
        &sentence_word_ranges,
        &media_paths,
        actual_total,
        &mixed_full_audio,
        &caption_style,
        out_width, out_height,
        total_audio_dur,
        vibe,
    )
    .await?;
    Ok(())
}

/// Shared tail of the studio pipeline: trims each scene to its sentence
/// duration, concatenates them, burns ASS captions, renders the final video
/// with audio + watermark, and inserts the clip record. Used by both the
/// stock and AI B-roll modes.
#[allow(clippy::too_many_arguments)]
async fn finish_studio_render(
    db: &DbPool,
    cfg: &PipelineConfig,
    output_dir: &Path,
    task_id_str: &str,
    items: &[ScriptItem],
    word_timestamps: &[WordTimestamp],
    sentence_word_ranges: &[WordRange],
    media_paths: &[PathBuf],
    actual_total: usize,
    mixed_full_audio: &Path,
    caption_style: &pipeline::caption::CaptionStyle,
    out_width: u32,
    out_height: u32,
    total_audio_dur: f64,
    vibe: &str,
) -> anyhow::Result<PathBuf> {
    let watermark_pos = &cfg.watermark_position;
    let watermark_opacity = cfg.watermark_opacity;
    let mut temp_files: Vec<PathBuf> = vec![mixed_full_audio.to_path_buf()];

    // Trim each media clip to its sentence duration, then concat into single video
    emit_progress(db, task_id_str, 60, "Assembling video scenes...", "processing").await;
    let mut trimmed_clips: Vec<PathBuf> = Vec::new();
    for i in 0..actual_total {
        if i >= sentence_word_ranges.len() || i >= items.len() {
            break;
        }
        let range = &sentence_word_ranges[i];
        let dur = (range.end_time - range.start_time).max(1.0);
        let trimmed = output_dir.join(format!("trimmed_{:02}.mp4", i + 1));
        match trim_clip_to_duration(&media_paths[i], &trimmed, dur, out_width, out_height, vibe).await {
            Ok(()) => {
                temp_files.push(trimmed.clone());
                trimmed_clips.push(trimmed);
            }
            Err(e) => {
                warn!("Trim failed for scene {}: {}", i + 1, e);
                // Create blank placeholder
                let placeholder = output_dir.join(format!("blank_{:02}.mp4", i + 1));
                let dur_str = format!("{:.3}", dur);
                let vf = format!("color=c=black:s={}x{}:d={}", out_width, out_height, dur_str);
                let _ = Command::new("ffmpeg")
                    .args(["-y", "-f", "lavfi", "-i", &vf])
                    .args(["-c:v", "libx264", "-preset", "fast", "-crf", "23", "-pix_fmt", "yuv420p"])
                    .arg(placeholder.to_str().unwrap())
                    .status().await;
                if placeholder.exists() {
                    temp_files.push(placeholder.clone());
                    trimmed_clips.push(placeholder);
                }
            }
        }
    }

    if trimmed_clips.is_empty() {
        anyhow::bail!("No trimmed clips could be created");
    }

    let concated_video = output_dir.join("concated_video.mp4");
    concat_clips(&trimmed_clips, &concated_video).await
        .map_err(|e| anyhow::anyhow!("Video concatenation failed: {}", e))?;
    temp_files.push(concated_video.clone());

    // Generate global ASS captions for entire video
    let ass_path = output_dir.join("captions.ass");
    if cfg.add_subtitles {
        generate_global_ass(
            &ass_path,
            word_timestamps,
            sentence_word_ranges,
            items,
            caption_style,
            cfg.auto_emojis,
            out_width,
            out_height,
        ).await?;
        temp_files.push(ass_path.clone());
    }

    // Final ffmpeg — burn captions + add audio + watermark in one pass
    emit_progress(db, task_id_str, 85, "Rendering final video...", "processing").await;
    let final_output = output_dir.join("final_video.mp4");

    let actual_watermark: Option<PathBuf> = {
        let p_png = output_dir.join("watermark.png");
        let p_webp = output_dir.join("watermark.webp");
        let p_jpg = output_dir.join("watermark.jpg");
        let p_jpeg = output_dir.join("watermark.jpeg");
        if p_png.exists() { Some(p_png) }
        else if p_webp.exists() { Some(p_webp) }
        else if p_jpg.exists() { Some(p_jpg) }
        else if p_jpeg.exists() { Some(p_jpeg) }
        else { cfg.watermark_path.as_ref().map(PathBuf::from).filter(|p| p.exists()) }
    };

    let has_watermark = !watermark_pos.is_empty() && watermark_opacity > 0.0 && actual_watermark.is_some();
    let wm_path = actual_watermark.unwrap_or_default();

    let mut ffmpeg_args = vec![
        "-y".to_string(),
        "-i".to_string(), concated_video.to_str().unwrap().to_string(),
        "-i".to_string(), mixed_full_audio.to_str().unwrap().to_string(),
    ];
    if has_watermark {
        ffmpeg_args.push("-loop".to_string());
        ffmpeg_args.push("1".to_string());
        ffmpeg_args.push("-i".to_string());
        ffmpeg_args.push(wm_path.to_str().unwrap().to_string());
    }

    let mut filter_parts: Vec<String> = Vec::new();

    // Caption burn
    if cfg.add_subtitles && ass_path.exists() {
        let ass_escaped = ass_path.to_str().unwrap()
            .replace('\\', "/")
            .replace(':', "\\:");
        filter_parts.push(format!("[0:v]subtitles=filename='{}'[vcap]", ass_escaped));
    } else {
        filter_parts.push("[0:v]null[vcap]".to_string());
    }

    // Audio mapping
    filter_parts.push("[1:a]acopy[a]".to_string());

    // Watermark overlay
    if has_watermark {
        let alpha = if watermark_opacity > 1.0 { watermark_opacity / 100.0 } else { watermark_opacity };
        let alpha = format!("{:.2}", alpha);
        let pos = match watermark_pos.as_str() {
            "top_left" => "30:30",
            "top_right" => "W-w-30:30",
            "bottom_left" => "30:H-h-30",
            "bottom_right" => "W-w-30:H-h-30",
            _ => "W-w-30:30",
        };
        // Scale logo to ~180px width for 1080p canvas, apply alpha transparency, then overlay
        let target_wm_width = (out_width as f64 * 0.18).round() as u32;
        filter_parts.push(format!(
            "[2:v]scale={}:-1,format=rgba,colorchannelmixer=aa={}[wm];[vcap][wm]overlay={}[vout]",
            target_wm_width, alpha, pos
        ));
    } else {
        filter_parts.push("[vcap]null[vout]".to_string());
    }

    let filter_complex = filter_parts.join("; ");
    ffmpeg_args.push("-filter_complex".to_string());
    ffmpeg_args.push(filter_complex);
    ffmpeg_args.push("-map".to_string());
    ffmpeg_args.push("[vout]".to_string());
    ffmpeg_args.push("-map".to_string());
    ffmpeg_args.push("[a]".to_string());
    ffmpeg_args.extend([
        "-c:v".to_string(), "libx264".to_string(),
        "-preset".to_string(), "medium".to_string(),
        "-crf".to_string(), "18".to_string(),
        "-c:a".to_string(), "aac".to_string(),
        "-b:a".to_string(), "192k".to_string(),
        "-pix_fmt".to_string(), "yuv420p".to_string(),
        "-movflags".to_string(), "+faststart".to_string(),
        "-shortest".to_string(),
        final_output.to_str().unwrap().to_string(),
    ]);

    let output = Command::new("ffmpeg")
        .args(&ffmpeg_args)
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn ffmpeg for final render: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Final video render ffmpeg failed: {}", stderr.trim());
    }

    if !final_output.exists() {
        anyhow::bail!("Final video was not created");
    }

    // Insert the final merged video as a single clip record
    let clip_id = Uuid::new_v4().to_string();
    let filename = "final_video.mp4".to_string();
    let all_sentences: Vec<&str> = items.iter().map(|i| i.sentence.as_str()).collect();
    let full_script_joined2 = all_sentences.join(" ");
    let hook_title = all_sentences.first().unwrap_or(&"");

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
    .bind(&filename)
    .bind(final_output.to_str().unwrap())
    .bind("0:00")
    .bind(format!("0:{:02}", total_audio_dur as i32))
    .bind(total_audio_dur)
    .bind(&full_script_joined2)
    .bind(1.0)
    .bind(100)
    .bind(100)
    .bind(100)
    .bind(100)
    .bind(100)
    .bind(Some("intro".to_string()))
    .bind(Some(hook_title.to_string()))
    .bind(Some("Studio task".to_string()))
    .execute(db)
    .await?;

    // Final progress
    emit_progress(db, task_id_str, 100, "Video ready!", "completed").await;

    // Cleanup temp files
    for f in &temp_files {
        if f != &final_output {
            tokio::fs::remove_file(f).await.ok();
        }
    }
    for c in &trimmed_clips {
        if c != &final_output {
            tokio::fs::remove_file(c).await.ok();
        }
    }
    if concated_video != final_output {
        tokio::fs::remove_file(&concated_video).await.ok();
    }

    info!("Studio task {} completed — {} scenes → 1 final video ({:.1}s)", task_id_str, actual_total, total_audio_dur);
    Ok(final_output)
}

/// Create a video clip from media of exact duration (no audio)
async fn trim_clip_to_duration(
    media_path: &Path,
    output: &Path,
    duration: f64,
    width: u32,
    height: u32,
    vibe: &str,
) -> anyhow::Result<()> {
    let ext = media_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4").to_lowercase();
    let is_video = matches!(ext.as_str(), "mp4" | "mov" | "webm" | "avi" | "mkv");
    let dur_str = format!("{:.3}", duration);
    let vibe_filter = match vibe {
        "lofi" => ",colorchannelmixer=.8:.2:.1:.1:.2:.8:.1:.1:.1:.2:.8",
        "futuristic" => ",colorchannelmixer=1.2:0:0:0:0:1.2:0:0:0:0:1.2,hue=45",
        "black_and_white" => ",hue=s=0,colorchannelmixer=.3:.4:.3:0:.3:.4:.3:0:.3:.4:.3",
        _ => "",
    };

    let status = if is_video {
        let vf = format!(
            "scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}{}",
            width, height, width, height, vibe_filter
        );
        Command::new("ffmpeg")
            .args(["-y", "-stream_loop", "-1", "-i", media_path.to_str().unwrap()])
            .args(["-vf", &vf, "-t", &dur_str])
            .args(["-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p", "-an", "-r", "30"])
            .arg(output.to_str().unwrap())
            .status().await
    } else {
        let fps = 30;
        let total_frames = (duration * fps as f64).max(1.0) as u32;
        let vf = format!(
            "scale={}x{}:force_original_aspect_ratio=increase,zoompan=z='min(zoom+0.005,1.3)':d={}:s={}x{}{}",
            width, height, total_frames, width, height, vibe_filter
        );
        Command::new("ffmpeg")
            .args(["-y", "-loop", "1", "-i", media_path.to_str().unwrap()])
            .args(["-vf", &vf, "-t", &dur_str])
            .args(["-c:v", "libx264", "-preset", "fast", "-crf", "18", "-pix_fmt", "yuv420p", "-r", "30"])
            .arg(output.to_str().unwrap())
            .status().await
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("FFmpeg trim clip failed for {:?} (dur={})", media_path, dur_str),
    }
}

/// Concat multiple video-only clips into one continuous video
async fn concat_clips(inputs: &[PathBuf], output: &Path) -> anyhow::Result<()> {
    let concat_file = output.parent().unwrap_or(Path::new(".")).join("concat_media.txt");
    {
        use std::fmt::Write;
        let mut content = String::new();
        for path in inputs {
            let abs = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
            let p = abs.to_str().unwrap_or_default().replace('\\', "/");
            let _ = writeln!(content, "file '{}'", p.replace('\'', "'\\''"));
        }
        tokio::fs::write(&concat_file, content.as_bytes()).await.ok();
    }

    let status = Command::new("ffmpeg")
        .args(["-y", "-f", "concat", "-safe", "0", "-i", concat_file.to_str().unwrap()])
        .args(["-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p", "-an"])
        .arg(output.to_str().unwrap())
        .status().await;

    tokio::fs::remove_file(&concat_file).await.ok();

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("FFmpeg concat clips failed"),
    }
}

/// Generate a global ASS subtitle file for the entire video using absolute word timestamps
async fn generate_global_ass(
    ass_path: &Path,
    word_timestamps: &[WordTimestamp],
    sentence_ranges: &[WordRange],
    items: &[ScriptItem],
    style: &pipeline::caption::CaptionStyle,
    auto_emojis: bool,
    width: u32,
    height: u32,
) -> anyhow::Result<()> {
    use crate::pipeline::caption::{ass_timestamp, escape_ass};
    let y_pos = (height as f64 * style.position_y_frac) as u32;

    let mut ass = format!(r#"[Script Info]
ScriptType: v4.00+
PlayResX: {}
PlayResY: {}
WrapStyle: 2
ScaledBorderAndShadow: yes

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,{},{},{},&H00000000,{},{},1,0,0,0,100,100,0,0,1,{},0,5,60,60,60,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"#,
        width, height,
        style.font_family, style.font_size,
        style.primary_color,
        style.stroke_color,
        if style.background { "&H99000000" } else { "&H00000000" },
        style.stroke_width,
    );

    for (item, range) in items.iter().zip(sentence_ranges.iter()) {
        let seg_words: Vec<&WordTimestamp> = word_timestamps[range.start_idx..range.end_idx.min(word_timestamps.len())].iter().collect();
        if seg_words.is_empty() {
            continue;
        }

        let emoji = if auto_emojis { get_sentence_emoji(&item.sentence) } else { None };

        // Group words into lines by max_words_per_line
        let mut line_groups: Vec<Vec<&WordTimestamp>> = Vec::new();
        let mut current_line: Vec<&WordTimestamp> = Vec::new();
        for w in &seg_words {
            current_line.push(w);
            if current_line.len() >= style.max_words_per_line {
                line_groups.push(std::mem::take(&mut current_line));
            }
        }
        if !current_line.is_empty() {
            line_groups.push(current_line);
        }

        for line_words in &line_groups {
            if style.word_pop {
                for (i, target) in line_words.iter().enumerate() {
                    let ws = target.start.max(0.0);
                    let we = target.end.max(ws + 0.1);
                    let mut line_formatted = String::new();
                    for (j, w) in line_words.iter().enumerate() {
                        let mut text = if style.uppercase { w.word.to_uppercase() } else { w.word.clone() };
                        if let Some(e) = emoji {
                            if i == line_words.len() - 1 && j == line_words.len() - 1 {
                                text.push(' ');
                                text.push_str(e);
                            }
                        }
                        if i == j {
                            line_formatted.push_str(&format!(
                                "{{\\c{}\\fscx110\\fscy110}}{}{{\\r}} ",
                                style.highlight_color,
                                escape_ass(&text)
                            ));
                        } else {
                            line_formatted.push_str(&format!(
                                "{{\\c{}}}{} ",
                                style.primary_color,
                                escape_ass(&text)
                            ));
                        }
                    }
                    ass.push_str(&format!(
                        "Dialogue: 0,{},{},Default,,0,0,0,,{{\\pos({},{})}}{}\n",
                        ass_timestamp(ws), ass_timestamp(we),
                        width / 2, y_pos,
                        line_formatted.trim_end()
                    ));
                }
            } else {
                let start = line_words[0].start.max(0.0);
                let end = line_words.last().unwrap().end.max(start + 0.1);
                let mut text_parts: Vec<String> = line_words.iter().map(|w| {
                    if style.uppercase { w.word.to_uppercase() } else { w.word.clone() }
                }).collect();
                if let Some(e) = emoji {
                    if let Some(last) = text_parts.last_mut() {
                        last.push(' ');
                        last.push_str(e);
                    }
                }
                let full_line = text_parts.join(" ");
                ass.push_str(&format!(
                    "Dialogue: 0,{},{},Default,,0,0,0,,{{\\pos({},{})\\c{}}}{}\n",
                    ass_timestamp(start), ass_timestamp(end),
                    width / 2, y_pos,
                    style.primary_color,
                    escape_ass(&full_line)
                ));
            }
        }
    }

    tokio::fs::write(ass_path, ass.as_bytes()).await?;
    Ok(())
}



/// Get audio duration in seconds via ffprobe
async fn get_audio_duration(path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-show_entries", "format=duration",
            "-of", "csv=p=0",
            path.to_str().unwrap(),
        ])
        .output()
        .await
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.trim().parse::<f64>().ok()
}

/// Generate a background music track using FFmpeg tones
async fn generate_bgm(output: &Path, style: &str, duration: f64) -> anyhow::Result<()> {
    let dur_str = format!("{}", duration);
    let (freq, amp) = match style {
        "upbeat" => (440.0, 0.03),
        "chill" => (220.0, 0.02),
        "cinematic" => (110.0, 0.04),
        _ => (0.0, 0.0),
    };
    if freq == 0.0 {
        anyhow::bail!("Unknown BGM style: {}", style);
    }
    let filter = format!("aevalsrc=sin({}*2*PI*t)*{}:d={}", freq, amp, dur_str);
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "lavfi",
            "-i", &filter,
            "-c:a", "libmp3lame",
            "-b:a", "128k",
            output.to_str().unwrap(),
        ])
        .status().await?;
    if !status.success() {
        anyhow::bail!("FFmpeg BGM generation failed");
    }
    Ok(())
}

/// Mix TTS audio with background music (BGM at reduced volume)
async fn mix_audio(tts_path: &Path, bgm_path: &Path, output: &Path) -> anyhow::Result<()> {
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", tts_path.to_str().unwrap(),
            "-i", bgm_path.to_str().unwrap(),
            "-filter_complex",
            "[1:a]volume=0.15[bgm];[0:a][bgm]amix=inputs=2:duration=first:dropout_transition=2",
            "-c:a", "libmp3lame",
            "-b:a", "128k",
            output.to_str().unwrap(),
        ])
        .status().await?;
    if !status.success() {
        anyhow::bail!("FFmpeg audio mix failed");
    }
    Ok(())
}

/// Stitch background media + audio into a single video
async fn stitch_media_audio(
    media_path: &Path,
    audio_path: &Path,
    output: &Path,
    audio_duration: f64,
    width: u32,
    height: u32,
    vibe: &str,
) -> anyhow::Result<()> {
    let ext = media_path.extension().and_then(|e| e.to_str()).unwrap_or("mp4").to_lowercase();
    let is_video = matches!(ext.as_str(), "mp4" | "mov" | "webm" | "avi" | "mkv");

    // Vibe-based color grading filter
    let vibe_filter = match vibe {
        "lofi" => ",colorchannelmixer=.8:.2:.1:.1:.2:.8:.1:.1:.1:.2:.8",
        "futuristic" => ",colorchannelmixer=1.2:0:0:0:0:1.2:0:0:0:0:1.2,hue=45",
        "black_and_white" => ",hue=s=0,colorchannelmixer=.3:.4:.3:0:.3:.4:.3:0:.3:.4:.3",
        _ => "",
    };

    let status = if is_video {
        let vf = format!(
            "scale={}:{}:force_original_aspect_ratio=increase,crop={}:{}{}",
            width, height, width, height, vibe_filter
        );
        Command::new("ffmpeg")
            .args([
                "-y",
                "-i", media_path.to_str().unwrap(),
                "-i", audio_path.to_str().unwrap(),
                "-map", "0:v:0",
                "-map", "1:a:0",
                "-c:v", "libx264",
                "-preset", "fast",
                "-crf", "18",
                "-vf", &vf,
                "-c:a", "aac",
                "-shortest",
                "-movflags", "+faststart",
                output.to_str().unwrap(),
            ])
            .status().await
    } else {
        // Photo: create video with Ken Burns zoom-in effect
        let dur_str = format!("{}", audio_duration);
        let fps = 30;
        let total_frames = (audio_duration * fps as f64) as u32;
        let vf = format!(
            "scale={}x{}:force_original_aspect_ratio=increase,zoompan=z='min(zoom+0.005,1.3)':d={}:s={}x{}{}",
            width, height, total_frames.max(1), width, height, vibe_filter
        );
        Command::new("ffmpeg")
            .args([
                "-y",
                "-loop", "1",
                "-i", media_path.to_str().unwrap(),
                "-i", audio_path.to_str().unwrap(),
                "-map", "0:v:0",
                "-map", "1:a:0",
                "-c:v", "libx264",
                "-preset", "fast",
                "-crf", "18",
                "-vf", &vf,
                "-c:a", "aac",
                "-t", &dur_str,
                "-shortest",
                "-pix_fmt", "yuv420p",
                "-movflags", "+faststart",
                output.to_str().unwrap(),
            ])
            .status().await
    };

    match status {
        Ok(s) if s.success() => Ok(()),
        _ => anyhow::bail!("FFmpeg stitch failed for {:?} + {:?}", media_path, audio_path),
    }
}

/// Apply watermark overlay using FFmpeg
async fn apply_watermark(input: &Path, output: &Path, position: &str, opacity: f64, watermark_path: &Path) -> anyhow::Result<()> {
    let pos_filter = match position {
        "top_left" => "10:10",
        "top_right" => "W-w-10:10",
        "bottom_left" => "10:H-h-10",
        "bottom_right" => "W-w-10:H-h-10",
        _ => "W-w-10:10",
    };
    let alpha = format!("{}", opacity / 100.0);
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", input.to_str().unwrap(),
            "-i", watermark_path.to_str().unwrap(),
            "-filter_complex",
            &format!("[1:v]format=rgba,colorkey=0x000000:0.1:0.0,colorchannelmixer=aa={}[wm];[0:v][wm]overlay={}", alpha, pos_filter),
            "-c:v", "libx264",
            "-preset", "fast",
            "-crf", "18",
            "-c:a", "copy",
            output.to_str().unwrap(),
        ])
        .status().await?;
    if !status.success() {
        anyhow::bail!("FFmpeg watermark overlay failed");
    }
    Ok(())
}

/// Extract a segment of audio from a file using ffmpeg
async fn extract_audio_segment(input: &Path, start: f64, end: f64, output: &Path) -> anyhow::Result<()> {
    let duration = end - start;
    let status = Command::new("ffmpeg")
        .args([
            "-y",
            "-i", input.to_str().unwrap(),
            "-ss", &format!("{:.3}", start),
            "-t", &format!("{:.3}", duration),
            "-c", "copy",
            output.to_str().unwrap(),
        ])
        .status()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to spawn ffmpeg for audio extract: {}", e))?;

    if !status.success() {
        anyhow::bail!("ffmpeg audio segment extraction failed");
    }
    Ok(())
}

/// Build a simple word-level array from a sentence for caption burning
fn build_simple_words(sentence: &str, total_duration: f64) -> Vec<crate::pipeline::transcribe::DeepgramWord> {
    let words: Vec<&str> = sentence.split_whitespace().collect();
    let n = words.len().max(1) as f64;
    let word_dur = total_duration / n;
    words.iter().enumerate().map(|(i, w)| {
        let start = i as f64 * word_dur;
        crate::pipeline::transcribe::DeepgramWord {
            word: w.to_string(),
            start,
            end: start + word_dur,
            confidence: 0.99,
            punctuated_word: Some(w.to_string()),
            speaker: None,
        }
    }).collect()
}

fn get_sentence_emoji(text: &str) -> Option<&'static str> {
    let pairs = [
        ("love", "❤️"), ("fire", "🔥"), ("money", "💰"), ("mind", "🧠"),
        ("brain", "🧠"), ("rocket", "🚀"), ("fast", "⚡"), ("power", "💪"),
        ("strong", "💪"), ("winner", "🏆"), ("win", "🏆"), ("goal", "🎯"),
        ("star", "⭐"), ("shining", "✨"), ("sparkle", "✨"), ("magic", "✨"),
        ("heart", "❤️"), ("cool", "😎"), ("awesome", "🔥"), ("amazing", "🔥"),
        ("crazy", "🤯"), ("wow", "😮"), ("shock", "😱"), ("laugh", "😂"),
        ("funny", "😂"), ("sad", "😢"), ("cry", "😭"), ("idea", "💡"),
        ("lightbulb", "💡"), ("book", "📚"), ("music", "🎵"), ("camera", "📷"),
        ("video", "🎥"), ("film", "🎬"), ("game", "🎮"), ("phone", "📱"),
        ("computer", "💻"), ("code", "💻"), ("web", "🌐"), ("world", "🌍"),
        ("travel", "✈️"), ("car", "🚗"), ("food", "🍔"), ("coffee", "☕"),
        ("sun", "☀️"), ("moon", "🌙"), ("nature", "🌿"), ("dog", "🐶"),
        ("cat", "🐱"), ("robot", "🤖"), ("alien", "👽"), ("100", "💯"),
        ("perfect", "💯"), ("time", "⏰"), ("clock", "⏰"),
    ];
    let lower = text.to_lowercase();
    for (keyword, emoji) in &pairs {
        if lower.contains(keyword) {
            return Some(emoji);
        }
    }
    None
}
