use macroquad::prelude::*;
use std::time::Instant;
use std::fs;
use std::path::Path;
use crate::audio::AudioSystem;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HitJudgment {
    Perfect,  // 300 - ±40ms
    Great,    // 200 - ±75ms
    Good,     // 100 - ±110ms
    Ok,       // 50  - ±135ms
    Miss,
}

impl HitJudgment {
    pub fn from_timing(abs_diff: f32) -> Self {
        if abs_diff <= 0.040 {
            HitJudgment::Perfect
        } else if abs_diff <= 0.075 {
            HitJudgment::Great
        } else if abs_diff <= 0.110 {
            HitJudgment::Good
        } else if abs_diff <= 0.135 {
            HitJudgment::Ok
        } else {
            HitJudgment::Miss
        }
    }
    
    pub fn score_value(&self) -> i32 {
        match self {
            HitJudgment::Perfect => 300,
            HitJudgment::Great => 200,
            HitJudgment::Good => 100,
            HitJudgment::Ok => 50,
            HitJudgment::Miss => 0,
        }
    }
    
    pub fn color(&self) -> Color {
        match self {
            HitJudgment::Perfect => Color::new(1.0, 0.8, 0.0, 1.0),
            HitJudgment::Great => Color::new(0.0, 1.0, 0.5, 1.0),
            HitJudgment::Good => Color::new(0.3, 0.8, 1.0, 1.0),
            HitJudgment::Ok => Color::new(0.7, 0.7, 0.7, 1.0),
            HitJudgment::Miss => RED,
        }
    }
    
    pub fn text(&self) -> &'static str {
        match self {
            HitJudgment::Perfect => "PERFECT",
            HitJudgment::Great => "GREAT",
            HitJudgment::Good => "GOOD",
            HitJudgment::Ok => "OK",
            HitJudgment::Miss => "MISS",
        }
    }
}

pub struct Note {
    pub start_time: f32,
    pub end_time: f32,
    pub lane: usize,
    pub hit: bool,
    pub missed: bool,
    
    // LN-specific state
    pub is_ln: bool,
    pub ln_head_hit: bool,
    pub ln_hold_broken: bool,
    pub ln_completed: bool,
    pub ln_head_judgment: Option<HitJudgment>,
    pub ln_tail_judgment: Option<HitJudgment>,
    
    // Audio tracking for sliders
    pub slider_sound_playing: bool,
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
    pub last_input_delay: f32,
    pub hit_counts: HitCounts,
    pub song_finished: bool,
    pub song_duration: f32,
    pub bg_texture: Option<Texture2D>,
    pub song_name: String,
    pub paused: bool,
    pub pause_start: Option<Instant>,
    pub total_pause_time: f32,
    pub speed_change_time: f32,
    pub speed_display_text: String,
    pub audio: Option<AudioSystem>,
}

pub struct HitCounts {
    pub perfect: i32,
    pub great: i32,
    pub good: i32,
    pub ok: i32,
    pub miss: i32,
}

#[derive(Clone)]
pub struct GameOptions {
    pub keys_2k: [KeyCode; 2],
    pub keys_4k: [KeyCode; 4],
    pub reverse_mode: bool,
    pub scroll_speed: i32, // 1-40, osu!mania standard
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            keys_2k: [KeyCode::D, KeyCode::K],
            keys_4k: [KeyCode::D, KeyCode::F, KeyCode::J, KeyCode::K],
            reverse_mode: false,
            scroll_speed: 20, // Default osu!mania speed
        }
    }
}

impl GameOptions {
    const CONFIG_FILE: &'static str = "rustania_config.txt";
    
    /// Load settings from file
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if !Path::new(Self::CONFIG_FILE).exists() {
            return Ok(Self::default());
        }
        
        let content = fs::read_to_string(Self::CONFIG_FILE)?;
        let mut options = Self::default();
        
        for line in content.lines() {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() != 2 { continue; }
            
            let key = parts[0].trim();
            let value = parts[1].trim();
            
            match key {
                "reverse_mode" => {
                    options.reverse_mode = value == "true";
                }
                "key_2k_0" => {
                    if let Ok(keycode) = Self::parse_keycode(value) {
                        options.keys_2k[0] = keycode;
                    }
                }
                "key_2k_1" => {
                    if let Ok(keycode) = Self::parse_keycode(value) {
                        options.keys_2k[1] = keycode;
                    }
                }
                "key_4k_0" => {
                    if let Ok(keycode) = Self::parse_keycode(value) {
                        options.keys_4k[0] = keycode;
                    }
                }
                "key_4k_1" => {
                    if let Ok(keycode) = Self::parse_keycode(value) {
                        options.keys_4k[1] = keycode;
                    }
                }
                "key_4k_2" => {
                    if let Ok(keycode) = Self::parse_keycode(value) {
                        options.keys_4k[2] = keycode;
                    }
                }
                "key_4k_3" => {
                    if let Ok(keycode) = Self::parse_keycode(value) {
                        options.keys_4k[3] = keycode;
                    }
                }
                "scroll_speed" => {
                    if let Ok(speed) = value.parse::<i32>() {
                        options.scroll_speed = speed.clamp(1, 40);
                    }
                }
                _ => {}
            }
        }
        
        Ok(options)
    }
    
    /// Save settings to file
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = format!(
            "reverse_mode={}\nkey_2k_0={}\nkey_2k_1={}\nkey_4k_0={}\nkey_4k_1={}\nkey_4k_2={}\nkey_4k_3={}\nscroll_speed={}\n",
            self.reverse_mode,
            Self::keycode_to_string(self.keys_2k[0]),
            Self::keycode_to_string(self.keys_2k[1]),
            Self::keycode_to_string(self.keys_4k[0]),
            Self::keycode_to_string(self.keys_4k[1]),
            Self::keycode_to_string(self.keys_4k[2]),
            Self::keycode_to_string(self.keys_4k[3]),
            self.scroll_speed,
        );
        
        fs::write(Self::CONFIG_FILE, content)?;
        Ok(())
    }
    
    fn keycode_to_string(key: KeyCode) -> String {
        format!("{:?}", key)
    }
    
    fn parse_keycode(s: &str) -> Result<KeyCode, ()> {
        // Parse common keys
        match s {
            "A" => Ok(KeyCode::A), "B" => Ok(KeyCode::B), "C" => Ok(KeyCode::C),
            "D" => Ok(KeyCode::D), "E" => Ok(KeyCode::E), "F" => Ok(KeyCode::F),
            "G" => Ok(KeyCode::G), "H" => Ok(KeyCode::H), "I" => Ok(KeyCode::I),
            "J" => Ok(KeyCode::J), "K" => Ok(KeyCode::K), "L" => Ok(KeyCode::L),
            "M" => Ok(KeyCode::M), "N" => Ok(KeyCode::N), "O" => Ok(KeyCode::O),
            "P" => Ok(KeyCode::P), "Q" => Ok(KeyCode::Q), "R" => Ok(KeyCode::R),
            "S" => Ok(KeyCode::S), "T" => Ok(KeyCode::T), "U" => Ok(KeyCode::U),
            "V" => Ok(KeyCode::V), "W" => Ok(KeyCode::W), "X" => Ok(KeyCode::X),
            "Y" => Ok(KeyCode::Y), "Z" => Ok(KeyCode::Z),
            "Key0" => Ok(KeyCode::Key0), "Key1" => Ok(KeyCode::Key1),
            "Key2" => Ok(KeyCode::Key2), "Key3" => Ok(KeyCode::Key3),
            "Key4" => Ok(KeyCode::Key4), "Key5" => Ok(KeyCode::Key5),
            "Key6" => Ok(KeyCode::Key6), "Key7" => Ok(KeyCode::Key7),
            "Key8" => Ok(KeyCode::Key8), "Key9" => Ok(KeyCode::Key9),
            "Space" => Ok(KeyCode::Space),
            "LeftShift" => Ok(KeyCode::LeftShift),
            "RightShift" => Ok(KeyCode::RightShift),
            "LeftControl" => Ok(KeyCode::LeftControl),
            "RightControl" => Ok(KeyCode::RightControl),
            "LeftAlt" => Ok(KeyCode::LeftAlt),
            "RightAlt" => Ok(KeyCode::RightAlt),
            "Comma" => Ok(KeyCode::Comma),
            "Period" => Ok(KeyCode::Period),
            "Slash" => Ok(KeyCode::Slash),
            "Semicolon" => Ok(KeyCode::Semicolon),
            "Apostrophe" => Ok(KeyCode::Apostrophe),
            "LeftBracket" => Ok(KeyCode::LeftBracket),
            "RightBracket" => Ok(KeyCode::RightBracket),
            _ => Err(())
        }
    }
}