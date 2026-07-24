-- SQLite-compatible schema (replaces PostgreSQL original)
-- UUIDs stored as TEXT, timestamps as TEXT (ISO-8601), JSON as TEXT, booleans as INTEGER

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    status TEXT NOT NULL DEFAULT 'queued',
    progress INTEGER NOT NULL DEFAULT 0,
    progress_message TEXT,
    source_url TEXT NOT NULL,
    source_title TEXT,
    source_type TEXT NOT NULL DEFAULT 'youtube',
    aspect_ratio TEXT NOT NULL DEFAULT '9:16',
    num_clips INTEGER NOT NULL DEFAULT 5,
    font_family TEXT NOT NULL DEFAULT 'THEBOLDFONT',
    font_size INTEGER NOT NULL DEFAULT 32,
    font_color TEXT NOT NULL DEFAULT '#FFFFFF',
    caption_template TEXT NOT NULL DEFAULT 'default',
    add_subtitles INTEGER NOT NULL DEFAULT 1,
    include_broll INTEGER NOT NULL DEFAULT 0,
    processing_mode TEXT NOT NULL DEFAULT 'fast',
    cut_long_pauses INTEGER NOT NULL DEFAULT 0,
    pause_threshold_ms INTEGER NOT NULL DEFAULT 900,
    remove_filler_words INTEGER NOT NULL DEFAULT 0,
    filtered_words TEXT NOT NULL DEFAULT '[]',
    transcript_cache TEXT,
    stage_timings TEXT NOT NULL DEFAULT '{}',
    error_message TEXT,
    gemini_api_key TEXT,
    deepgram_api_key TEXT,
    cancel_requested INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS generated_clips (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    clip_order INTEGER NOT NULL DEFAULT 0,
    filename TEXT NOT NULL,
    file_path TEXT NOT NULL,
    start_time TEXT NOT NULL,
    end_time TEXT NOT NULL,
    duration REAL NOT NULL,
    transcript_text TEXT,
    relevance_score REAL NOT NULL DEFAULT 0.75,
    virality_score INTEGER NOT NULL DEFAULT 0,
    hook_score INTEGER NOT NULL DEFAULT 0,
    engagement_score INTEGER NOT NULL DEFAULT 0,
    value_score INTEGER NOT NULL DEFAULT 0,
    shareability_score INTEGER NOT NULL DEFAULT 0,
    hook_type TEXT,
    hook_title TEXT,
    reasoning TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS processing_cache (
    cache_key TEXT PRIMARY KEY,
    source_url TEXT NOT NULL,
    transcript_json TEXT,
    analysis_json TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_created_at ON tasks(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_generated_clips_task_id ON generated_clips(task_id);
CREATE INDEX IF NOT EXISTS idx_generated_clips_order ON generated_clips(task_id, clip_order);
