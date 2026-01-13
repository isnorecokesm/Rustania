use macroquad::prelude::*;
use std::time::Instant;

pub struct Note {
    pub start_time: f32,
    pub end_time: f32,
    pub lane: usize,
    pub hit: bool,
    pub missed: bool,
    pub ln_started: bool,      // Track if LN head was hit
    pub ln_completed: bool,    // Track if LN was held to end
}

pub struct GameState {
    pub notes: Vec<Note>,
    pub score: i32,
    pub combo: i32,
    pub last_judgment: &'static str,
    pub judgment_color: Color,
    pub judgment_time: f32,
    pub start_time: Instant,
    pub key_count: usize,
    pub last_input_delay: f32, // in milliseconds, positive = late, negative = early
    pub hit_counts: HitCounts,
    pub song_finished: bool,
    pub song_duration: f32,
    pub bg_texture: Option<Texture2D>,
}

pub struct HitCounts {
    pub perfect: i32,
    pub great: i32,
    pub good: i32,
    pub ok: i32,
    pub miss: i32,
}