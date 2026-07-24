// Crop module - smart vertical crop logic is handled in clip.rs via FFmpeg filters
// This module provides helper utilities for aspect ratio validation and dimension calculation.

pub fn parse_aspect_ratio(ar: &str) -> Option<(u32, u32)> {
    let parts: Vec<&str> = ar.split(':').collect();
    if parts.len() != 2 { return None; }
    let w: u32 = parts[0].parse().ok()?;
    let h: u32 = parts[1].parse().ok()?;
    Some((w, h))
}

pub fn output_dimensions(aspect_ratio: &str) -> (u32, u32) {
    match aspect_ratio {
        "9:16" => (1080, 1920),
        "1:1"  => (1080, 1080),
        "16:9" => (1920, 1080),
        "4:3"  => (1440, 1080),
        _      => (1920, 1080), // original - use source dims
    }
}
