pub mod pipeline;
pub use pipeline::PipelineConfig;

use novaclip_db::DbPool;
use std::path::PathBuf;
use tracing::{error, info, warn};
use uuid::Uuid;

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
                // Reuse the existing pool — fixes the original bug of creating a new pool on error
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
/// The SSE handler polls this table every second to stream events to clients.
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
    use pipeline::{
        analyze::analyze_transcript,
        caption::{burn_captions, get_caption_style, get_clip_words},
        clip::extract_clip,
        crop::output_dimensions,
        dedup::{dedup_segments, timestamp_to_seconds},
        download::{download_youtube, extract_audio, get_video_duration, resolve_upload_path},
        transcribe::{build_transcript_for_prompt, transcribe_with_deepgram},
    };

    let task_id_str = task_id.to_string();

    // Load task
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

    // Early cancel check
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
        filtered_words: serde_json::from_str::<Vec<String>>(&task.filtered_words)
            .unwrap_or_default(),
        output_dir: std::env::var("OUTPUT_DIR").unwrap_or_else(|_| "outputs".into()),
        temp_dir: std::env::var("TEMP_DIR").unwrap_or_else(|_| "temp".into()),
        gemini_api_key: task.gemini_api_key
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| std::env::var("GEMINI_API_KEY").unwrap_or_default()),
        gemini_model: std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-3.1-flash-lite".into()),
        deepgram_api_key: task.deepgram_api_key
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| std::env::var("DEEPGRAM_API_KEY").unwrap_or_default()),
        pexels_api_key: std::env::var("PEXELS_API_KEY").ok().filter(|s| !s.is_empty()),
    };

    // ===== STAGE 1: DOWNLOAD =====
    emit_progress(&db, &task_id_str, 5, "Downloading video...", "processing").await;

    let video_path = if cfg.source_type == "upload" {
        let p = resolve_upload_path(&cfg.url, &cfg.temp_dir);
        if !p.exists() {
            anyhow::bail!("Uploaded file not found: {}", p.display());
        }
        p
    } else {
        download_youtube(&cfg.url, &cfg.temp_dir).await?
    };

    if check_cancelled(&db, &task_id_str).await {
        return Ok(());
    }

    let duration = get_video_duration(&video_path).await?;
    info!("Video duration: {:.1}s", duration);

    // ===== STAGE 2: TRANSCRIBE =====
    emit_progress(&db, &task_id_str, 15, "Extracting audio...", "processing").await;
    let audio_path = extract_audio(&video_path, &cfg.temp_dir).await?;

    emit_progress(&db, &task_id_str, 25, "Transcribing audio...", "processing").await;
    let transcript = transcribe_with_deepgram(&audio_path, &cfg.deepgram_api_key).await?;
    let transcript_text = build_transcript_for_prompt(&transcript);

    // Cache transcript in DB
    sqlx::query(
        "UPDATE tasks SET transcript_cache = ?, updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&transcript_text)
    .bind(&task_id_str)
    .execute(&db)
    .await
    .ok();

    if check_cancelled(&db, &task_id_str).await {
        return Ok(());
    }

    // ===== STAGE 3: ANALYZE =====
    emit_progress(&db, &task_id_str, 40, "Analyzing virality with AI...", "processing").await;
    let analysis = analyze_transcript(
        &transcript_text,
        cfg.num_clips,
        &cfg.gemini_model,
        &cfg.gemini_api_key,
    )
    .await?;

    // ===== STAGE 4: DEDUP =====
    let deduped = dedup_segments(analysis.most_relevant_segments, cfg.num_clips as usize);
    info!("Selected {} clips after dedup", deduped.len());

    if check_cancelled(&db, &task_id_str).await {
        return Ok(());
    }

    // ===== STAGE 5: EXTRACT CLIPS =====
    let output_dir = PathBuf::from(&cfg.output_dir).join(task_id.to_string());
    tokio::fs::create_dir_all(&output_dir).await?;

    let (out_width, out_height) = output_dimensions(&cfg.aspect_ratio);
    let caption_style = get_caption_style(
        &cfg.caption_template,
        &cfg.font_family,
        cfg.font_size,
        &cfg.font_color,
    );
    let total_clips = deduped.len();

    for (i, seg) in deduped.iter().enumerate() {
        if check_cancelled(&db, &task_id_str).await {
            return Ok(());
        }

        let pct = 50 + (i * 45 / total_clips.max(1)) as i32;
        let msg = format!("Rendering clip {} of {}...", i + 1, total_clips);
        emit_progress(&db, &task_id_str, pct, &msg, "processing").await;

        let raw_clip_path = extract_clip(
            &video_path,
            &output_dir,
            i as i32,
            &seg.start_time,
            &seg.end_time,
            &cfg.aspect_ratio,
            &task_id.to_string(),
        )
        .await?;

        let final_path = if cfg.add_subtitles {
            let start_secs = timestamp_to_seconds(&seg.start_time);
            let end_secs = timestamp_to_seconds(&seg.end_time);
            let clip_words = get_clip_words(&transcript.words, start_secs, end_secs);
            match burn_captions(
                &raw_clip_path,
                &output_dir,
                &clip_words,
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
                    tokio::fs::remove_file(&raw_clip_path).await.ok();
                    p
                }
                Err(e) => {
                    warn!("Caption burn failed: {} — using raw clip", e);
                    raw_clip_path
                }
            }
        } else {
            raw_clip_path
        };

        let filename = final_path
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
        .bind(&task_id_str)
        .bind(i as i32)
        .bind(&filename)
        .bind(final_path.to_str().unwrap())
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
        .execute(&db)
        .await?;
    }

    // ===== DONE =====
    sqlx::query(
        "UPDATE tasks SET status = 'completed', progress = 100, progress_message = 'All clips ready!', completed_at = datetime('now'), updated_at = datetime('now') WHERE id = ?"
    )
    .bind(&task_id_str)
    .execute(&db)
    .await?;

    tokio::fs::remove_file(&audio_path).await.ok();
    info!("Task {} completed — {} clips generated", task_id, total_clips);
    Ok(())
}
