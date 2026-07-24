use crate::pipeline::TranscriptSegment;
use tracing::info;

/// Convert MM:SS or HH:MM:SS to seconds
pub fn timestamp_to_seconds(ts: &str) -> f64 {
    let parts: Vec<f64> = ts.split(':')
        .filter_map(|p| p.parse().ok())
        .collect();
    match parts.len() {
        2 => parts[0] * 60.0 + parts[1],
        3 => parts[0] * 3600.0 + parts[1] * 60.0 + parts[2],
        _ => 0.0,
    }
}

/// Remove overlapping clips — keep highest virality score when clips overlap
pub fn dedup_segments(mut segments: Vec<TranscriptSegment>, max_clips: usize) -> Vec<TranscriptSegment> {
    // Sort by virality score descending
    segments.sort_by(|a, b| b.virality.total_score.cmp(&a.virality.total_score));

    let mut kept: Vec<TranscriptSegment> = Vec::new();

    for seg in segments {
        let start = timestamp_to_seconds(&seg.start_time);
        let end = timestamp_to_seconds(&seg.end_time);

        // Validate: end must be after start, min 15 seconds
        if end <= start || (end - start) < 15.0 {
            info!("Skipping invalid segment [{} - {}] (duration {:.1}s)", seg.start_time, seg.end_time, end - start);
            continue;
        }

        // Check overlap with already-kept segments
        let overlaps = kept.iter().any(|k| {
            let ks = timestamp_to_seconds(&k.start_time);
            let ke = timestamp_to_seconds(&k.end_time);
            // Overlap if not fully before or fully after
            !(end <= ks || start >= ke)
        });

        if !overlaps {
            kept.push(seg);
            if kept.len() >= max_clips {
                break;
            }
        } else {
            info!("Deduped overlapping segment [{} - {}]", seg.start_time, seg.end_time);
        }
    }

    // Sort final set by start time
    kept.sort_by(|a, b| {
        timestamp_to_seconds(&a.start_time).partial_cmp(&timestamp_to_seconds(&b.start_time)).unwrap()
    });

    info!("Dedup result: {} clips kept", kept.len());
    kept
}
