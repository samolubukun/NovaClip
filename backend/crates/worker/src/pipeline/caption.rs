use anyhow::Result;
use std::path::{Path, PathBuf};
use tracing::info;
use crate::pipeline::dedup::timestamp_to_seconds;
use crate::pipeline::transcribe::DeepgramWord;

/// Caption template styles
pub struct CaptionStyle {
    pub font_family: String,
    pub font_size: i32,
    pub primary_color: String,   // ASS format: &HBBGGRR&
    pub highlight_color: String,
    pub stroke_color: String,
    pub stroke_width: i32,
    pub show_hook_title: bool,
    pub uppercase: bool,
    pub word_pop: bool,
    pub background: bool,
    pub glow: bool,
    pub max_words_per_line: usize,
    pub position_y_frac: f64,
}

/// Convert hex #RRGGBB to ASS &H00BBGGRR& format
fn hex_to_ass(hex: &str) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 { return "&H00FFFFFF&".to_string(); }
    let r = &h[0..2];
    let g = &h[2..4];
    let b = &h[4..6];
    format!("&H00{}{}{}&", b, g, r)
}

pub fn get_caption_style(template: &str, font_family: &str, font_size: i32, font_color: &str) -> CaptionStyle {
    match template {
        "bold" | "hormozi" => CaptionStyle {
            font_family: "THEBOLDFONT".to_string(),
            font_size: 48,
            primary_color: hex_to_ass("#FFFFFF"),
            highlight_color: hex_to_ass("#00FF66"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 5,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: false,
            max_words_per_line: 3,
            position_y_frac: 0.74,
        },
        "vibrant" | "mrbeast" => CaptionStyle {
            font_family: "THEBOLDFONT".to_string(),
            font_size: 52,
            primary_color: hex_to_ass("#FFFF00"),
            highlight_color: hex_to_ass("#FF2D2D"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 6,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: false,
            max_words_per_line: 3,
            position_y_frac: 0.70,
        },
        "tiktok" => CaptionStyle {
            font_family: "TikTokSans-Regular".to_string(),
            font_size: 44,
            primary_color: hex_to_ass("#FFFFFF"),
            highlight_color: hex_to_ass("#FE2C55"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 4,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: false,
            max_words_per_line: 4,
            position_y_frac: 0.78,
        },
        "neon" => CaptionStyle {
            font_family: "THEBOLDFONT".to_string(),
            font_size: 46,
            primary_color: hex_to_ass("#00FFFF"),
            highlight_color: hex_to_ass("#FF00FF"),
            stroke_color: hex_to_ass("#002A6B"),
            stroke_width: 3,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: true,
            max_words_per_line: 4,
            position_y_frac: 0.76,
        },
        "podcast" => CaptionStyle {
            font_family: "TikTokSans-Regular".to_string(),
            font_size: 40,
            primary_color: hex_to_ass("#FFFFFF"),
            highlight_color: hex_to_ass("#FFB800"),
            stroke_color: hex_to_ass("#1A1A1A"),
            stroke_width: 3,
            show_hook_title: false,
            uppercase: true,
            word_pop: false,
            background: true,
            glow: false,
            max_words_per_line: 5,
            position_y_frac: 0.80,
        },
        "minimal" => CaptionStyle {
            font_family: "TikTokSans-Regular".to_string(),
            font_size: 38,
            primary_color: hex_to_ass("#FFFFFF"),
            highlight_color: hex_to_ass("#FFFFFF"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 1,
            show_hook_title: false,
            uppercase: true,
            word_pop: false,
            background: true,
            glow: false,
            max_words_per_line: 6,
            position_y_frac: 0.82,
        },
        "cinematic" => CaptionStyle {
            font_family: "THEBOLDFONT".to_string(),
            font_size: 44,
            primary_color: hex_to_ass("#FFD700"),
            highlight_color: hex_to_ass("#FFFFFF"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 4,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: false,
            max_words_per_line: 4,
            position_y_frac: 0.75,
        },
        "cyber" => CaptionStyle {
            font_family: "THEBOLDFONT".to_string(),
            font_size: 48,
            primary_color: hex_to_ass("#39FF14"),
            highlight_color: hex_to_ass("#00FFFF"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 5,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: true,
            max_words_per_line: 3,
            position_y_frac: 0.76,
        },
        "clean" => CaptionStyle {
            font_family: "TikTokSans-Regular".to_string(),
            font_size: 42,
            primary_color: hex_to_ass("#FFFFFF"),
            highlight_color: hex_to_ass("#FFE000"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 3,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: true,
            glow: false,
            max_words_per_line: 4,
            position_y_frac: 0.78,
        },
        _ => CaptionStyle { // default
            font_family: font_family.to_string(),
            font_size: font_size.max(44),
            primary_color: hex_to_ass(font_color),
            highlight_color: hex_to_ass("#FFE000"),
            stroke_color: hex_to_ass("#000000"),
            stroke_width: 4,
            show_hook_title: false,
            uppercase: true,
            word_pop: true,
            background: false,
            glow: false,
            max_words_per_line: 4,
            position_y_frac: 0.80,
        }
    }
}

fn ass_timestamp(secs: f64) -> String {
    let s = secs.max(0.0);
    let h = (s / 3600.0) as u64;
    let m = ((s % 3600.0) / 60.0) as u64;
    let sec = s % 60.0;
    format!("{}:{:02}:{:05.2}", h, m, sec)
}

fn escape_ass(text: &str) -> String {
    text.replace('\\', "\\\\").replace('{', "\\{").replace('}', "\\}")
}

/// Burn ASS karaoke subtitles into a clip using FFmpeg
pub async fn burn_captions(
    input: &Path,
    output_dir: &Path,
    clip_words: &[&DeepgramWord],
    clip_start_secs: f64,
    style: &CaptionStyle,
    hook_title: Option<&str>,
    clip_duration: f64,
    width: u32,
    height: u32,
) -> Result<PathBuf> {
    tokio::fs::create_dir_all(output_dir).await?;
    let ass_path = output_dir.join(format!("captions_{}.ass", uuid::Uuid::new_v4()));
    let output_path = output_dir.join(format!("captioned_{}.mp4", uuid::Uuid::new_v4()));
    let y_pos = (height as f64 * style.position_y_frac) as u32;

    // Build ASS header
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

    // Hook title banner (only shown if show_hook_title is explicitly enabled)
    if style.show_hook_title {
        if let Some(title) = hook_title {
        let title_text = if style.uppercase { title.to_uppercase() } else { title.to_string() };
        let title_end = clip_duration.min(4.0);
        let title_y = (height as f64 * 0.07) as u32;
        ass.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{{\\pos({},{})\\c{}\\fs{}\\bord3\\shad2}}{}\n",
            ass_timestamp(0.0), ass_timestamp(title_end),
            width / 2, title_y,
            hex_to_ass("#FFE000"),
            (style.font_size as f64 * 0.85) as i32,
            escape_ass(&title_text)
        ));
    }
    }

    // Word-by-word karaoke captions
    let mut line_words: Vec<&DeepgramWord> = Vec::new();
    let mut line_start = 0.0f64;

    for word in clip_words {
        let ws = word.start - clip_start_secs;
        let we = word.end - clip_start_secs;
        if ws < 0.0 { continue; }

        line_words.push(word);

        if line_words.len() >= style.max_words_per_line {
            flush_caption_line(&mut ass, &line_words, clip_start_secs, &style, y_pos, width);
            line_words.clear();
        }
    }
    if !line_words.is_empty() {
        flush_caption_line(&mut ass, &line_words, clip_start_secs, &style, y_pos, width);
    }

    tokio::fs::write(&ass_path, &ass).await?;

    // Escape path for FFmpeg subtitles filter
    let ass_escaped = ass_path.to_str().unwrap()
        .replace('\\', "/")
        .replace(':', "\\:");

    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y", "-i", input.to_str().unwrap(),
            "-vf", &format!("subtitles=filename='{}'", ass_escaped),
            "-c:v", "libx264", "-preset", "fast", "-crf", "18",
            "-pix_fmt", "yuv420p", "-c:a", "copy",
            "-movflags", "+faststart",
            output_path.to_str().unwrap(),
        ])
        .status().await?;

    tokio::fs::remove_file(&ass_path).await.ok();

    if !status.success() {
        anyhow::bail!("FFmpeg subtitle burn failed");
    }
    Ok(output_path)
}

fn flush_caption_line(
    ass: &mut String,
    words: &[&DeepgramWord],
    clip_start: f64,
    style: &CaptionStyle,
    y_pos: u32,
    width: u32,
) {
    if words.is_empty() { return; }

    if style.word_pop {
        // Generate a dialogue line per word so the currently spoken word turns into highlight_color
        for (i, target_word) in words.iter().enumerate() {
            let ws = (target_word.start - clip_start).max(0.0);
            let we = (target_word.end - clip_start).max(ws + 0.1);

            let mut line_formatted = String::new();
            for (j, w) in words.iter().enumerate() {
                let raw_t = w.punctuated_word.as_deref().unwrap_or(&w.word);
                let text = if style.uppercase { raw_t.to_uppercase() } else { raw_t.to_string() };

                if i == j {
                    // Spoken word -> highlight color + slight scale up
                    line_formatted.push_str(&format!(
                        "{{\\c{}\\fscx110\\fscy110}}{}{{\\r}} ",
                        style.highlight_color,
                        escape_ass(&text)
                    ));
                } else {
                    // Regular word -> primary color
                    line_formatted.push_str(&format!(
                        "{{\\c{}}}{} ",
                        style.primary_color,
                        escape_ass(&text)
                    ));
                }
            }

            ass.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{{\\pos({},{})}}{}\n",
                ass_timestamp(ws),
                ass_timestamp(we),
                width / 2, y_pos,
                line_formatted.trim_end()
            ));
        }
    } else {
        // Static full line display
        let line_start = words.first().unwrap().start - clip_start;
        let line_end = words.last().unwrap().end - clip_start;
        let full_line: String = words.iter()
            .map(|w| {
                let t = w.punctuated_word.as_deref().unwrap_or(&w.word);
                if style.uppercase { t.to_uppercase() } else { t.to_string() }
            })
            .collect::<Vec<_>>().join(" ");

        ass.push_str(&format!(
            "Dialogue: 0,{},{},Default,,0,0,0,,{{\\pos({},{})\\c{}}}{}\n",
            ass_timestamp(line_start.max(0.0)),
            ass_timestamp(line_end.max(line_start + 0.1)),
            width / 2, y_pos,
            style.primary_color,
            escape_ass(&full_line)
        ));
    }
}

pub fn get_clip_words<'a>(
    all_words: &'a [crate::pipeline::transcribe::DeepgramWord],
    start_secs: f64,
    end_secs: f64,
) -> Vec<&'a DeepgramWord> {
    all_words.iter()
        .filter(|w| w.start >= start_secs && w.end <= end_secs + 0.5)
        .collect()
}
