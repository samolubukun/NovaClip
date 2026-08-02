use std::path::PathBuf;
use novaclip_worker::pipeline::PipelineConfig;
use novaclip_worker::pipeline::transcribe::{transcribe_with_whisper, diarize_words_local};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: test_diarize <audio_file>");
        return Ok(());
    }

    let audio_path = PathBuf::from(&args[1]);

    // Construct a dummy PipelineConfig
    let cfg = PipelineConfig {
        task_id: uuid::Uuid::new_v4(),
        url: "".into(),
        source_type: "".into(),
        aspect_ratio: "".into(),
        num_clips: 1,
        font_family: "".into(),
        font_size: 24,
        font_color: "".into(),
        caption_template: "".into(),
        add_subtitles: false,
        include_broll: false,
        processing_mode: "".into(),
        cut_long_pauses: false,
        pause_threshold_ms: 0,
        remove_filler_words: false,
        auto_vertical_reframe: false,
        reframe_preset: "".into(),
        reframe_frame_skip: 1,
        reframe_layout: "".into(),
        speaker_active_switch: false,
        split_divider: false,
        originality_boost: "".into(),
        translate_language: "".into(),
        giphy_api_key: None,
        filtered_words: vec![],
        output_dir: "".into(),
        temp_dir: "temp".into(),
        gemini_api_key: "".into(),
        gemini_model: "".into(),
        deepgram_api_key: "".into(),
        stt_provider: "".into(),
        vosk_model_path: "".into(),
        whisper_model_path: "models/ggml-base.bin".into(),
        pyannote_segmentation_model_path: "models/segmentation-3.0.onnx".into(),
        pyannote_embedding_model_path: "models/model.onnx".into(),
        pexels_api_key: None,
        pixabay_api_key: None,
        studio_payload: None,
        highlight_color: "".into(),
        caption_animation: "".into(),
        auto_emojis: false,
        watermark_position: "".into(),
        watermark_opacity: 1.0,
        watermark_path: None,
    };

    let whisper_model = PathBuf::from(&cfg.whisper_model_path);
    let seg_model = PathBuf::from(&cfg.pyannote_segmentation_model_path);
    let emb_model = PathBuf::from(&cfg.pyannote_embedding_model_path);

    println!("Transcribing audio with Whisper and Diarizing...");
    let mut transcript = transcribe_with_whisper(&audio_path, &whisper_model, &seg_model, &emb_model).await?;
    diarize_words_local(&audio_path, &mut transcript.words, &seg_model, &emb_model).await?;

    println!("\n--- RESULTS ---");
    for w in transcript.words {
        println!("[{:.2}s - {:.2}s] Speaker {:?}: {}", w.start, w.end, w.speaker, w.word);
    }
    
    Ok(())
}
