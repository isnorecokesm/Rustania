#![windows_subsystem = "windows"]

mod models;
mod parser;
mod game;
mod discord_rpc;
mod audio;

use macroquad::prelude::*;
use macroquad::ui::root_ui;
use rodio::{OutputStream, Sink};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use models::GameOptions;

#[macroquad::main("Rustania")]
async fn main() {
    let mut scene = "Menu";
    let mut state: Option<models::GameState> = None;
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let audio_sink: Arc<Mutex<Option<Sink>>> = Arc::new(Mutex::new(None));
    let mut selected_beatmap: Option<PathBuf> = None;
    let mut difficulties: Vec<parser::BeatmapInfo> = Vec::new();
    let mut key_mode = 4;
    let mut rpc = discord_rpc::RpcManager::new();
    rpc.update_idle();
    let _ = fs::create_dir_all("beatmaps");
    let mut song_finished_shown = false;
    
    // Load saved options or use defaults
    let mut options = GameOptions::load().unwrap_or_default();
    
    // Key remapping state
    let mut remapping_mode: Option<(usize, bool)> = None; // (key_index, is_2k_mode)

    loop {
        clear_background(BLACK);
        
        // Handle pause/resume for audio during gameplay
        if scene == "Playing" {
            if let Some(ref mut s) = state {
                if let Ok(sink_lock) = audio_sink.lock() {
                    if let Some(sink) = sink_lock.as_ref() {
                        if s.paused {
                            sink.pause();
                        } else {
                            sink.play();
                        }
                    }
                }
            }
        }
        
        match scene {
            "Menu" => {
                song_finished_shown = false;
                draw_rectangle(0.0, 0.0, 500.0, screen_height(), Color::new(0.1, 0.1, 0.1, 1.0));
                draw_text("RUSTANIA", 40.0, 60.0, 40.0, SKYBLUE);
                
                // Key mode selector
                draw_text(&format!("MODE: {}K", key_mode), 40.0, 110.0, 25.0, WHITE);
                if root_ui().button(vec2(40.0, 125.0), "2K") {
                    key_mode = 2;
                }
                if root_ui().button(vec2(100.0, 125.0), "4K") {
                    key_mode = 4;
                }
                
                if root_ui().button(vec2(40.0, 170.0), "OPTIONS") {
                    scene = "Options";
                }
                
                if root_ui().button(vec2(40.0, 210.0), "IMPORT .OSZ FILE") {
                    if let Some(path) = rfd::FileDialog::new().add_filter("osu", &["osz"]).pick_file() {
                        let _ = parser::import_osz(path);
                    }
                }

                draw_text("BEATMAPS:", 40.0, 270.0, 25.0, GRAY);
                
                if let Ok(entries) = fs::read_dir("beatmaps") {
                    for (i, entry) in entries.flatten().enumerate() {
                        let name = entry.file_name().into_string().unwrap_or_default();
                        let truncated = if name.len() > 35 {
                            format!("{}...", &name[..32])
                        } else {
                            name.clone()
                        };
                        
                        if root_ui().button(vec2(40.0, 300.0 + (i as f32 * 40.0)), truncated.as_str()) {
                            selected_beatmap = Some(entry.path());
                            if let Ok(diffs) = parser::get_difficulties(&entry.path()) {
                                difficulties = diffs;
                                scene = "DiffSelect";
                            }
                        }
                    }
                }
            }
            "Options" => {
                draw_rectangle(0.0, 0.0, 700.0, screen_height(), Color::new(0.1, 0.1, 0.1, 1.0));
                draw_text("OPTIONS", 40.0, 60.0, 40.0, SKYBLUE);
                
                if root_ui().button(vec2(40.0, 80.0), "< BACK") {
                    // Save options when leaving
                    let _ = options.save();
                    scene = "Menu";
                }
                
                // Reverse mode toggle
                draw_text("GAMEPLAY:", 40.0, 140.0, 30.0, WHITE);
                let reverse_text = if options.reverse_mode { "Reverse: ON (FNF style)" } else { "Reverse: OFF (Normal)" };
                if root_ui().button(vec2(40.0, 160.0), reverse_text) {
                    options.reverse_mode = !options.reverse_mode;
                }
                
                // 2K Key bindings
                draw_text("2K KEY BINDINGS:", 40.0, 230.0, 30.0, WHITE);
                for i in 0..2 {
                    let key_name = format!("Lane {}: {:?}", i + 1, options.keys_2k[i]);
                    let button_text = if remapping_mode == Some((i, true)) {
                        "Press any key...".to_string()
                    } else {
                        key_name
                    };
                    
                    if root_ui().button(vec2(40.0, 260.0 + (i as f32 * 40.0)), button_text.as_str()) {
                        remapping_mode = Some((i, true));
                    }
                }
                
                // 4K Key bindings
                draw_text("4K KEY BINDINGS:", 40.0, 380.0, 30.0, WHITE);
                for i in 0..4 {
                    let key_name = format!("Lane {}: {:?}", i + 1, options.keys_4k[i]);
                    let button_text = if remapping_mode == Some((i, false)) {
                        "Press any key...".to_string()
                    } else {
                        key_name
                    };
                    
                    if root_ui().button(vec2(40.0, 410.0 + (i as f32 * 40.0)), button_text.as_str()) {
                        remapping_mode = Some((i, false));
                    }
                }
                
                // Handle key remapping
                if let Some((key_index, is_2k)) = remapping_mode {
                    // Check for any key press
                    if let Some(pressed_key) = get_pressed_key() {
                        if is_2k {
                            options.keys_2k[key_index] = pressed_key;
                        } else {
                            options.keys_4k[key_index] = pressed_key;
                        }
                        remapping_mode = None;
                    }
                }
            }
            "DiffSelect" => {
                draw_rectangle(0.0, 0.0, 600.0, screen_height(), Color::new(0.1, 0.1, 0.1, 1.0));
                
                if let Some(ref bm_path) = selected_beatmap {
                    let bm_name = bm_path.file_name().unwrap().to_str().unwrap_or("Unknown");
                    draw_text("SELECT DIFFICULTY", 40.0, 60.0, 30.0, SKYBLUE);
                    draw_text(bm_name, 40.0, 95.0, 20.0, GRAY);
                    draw_text(&format!("Playing in: {}K mode", key_mode), 40.0, 120.0, 20.0, YELLOW);
                    if options.reverse_mode {
                        draw_text("Reverse Mode: ON", 40.0, 145.0, 20.0, ORANGE);
                    }
                }
                
                if root_ui().button(vec2(40.0, 170.0), "< BACK TO MENU") {
                    scene = "Menu";
                    rpc.update_idle();
                }

                for (i, diff) in difficulties.iter().enumerate() {
                    if root_ui().button(vec2(40.0, 220.0 + (i as f32 * 40.0)), diff.version.as_str()) {
                        if let Ok((s, sink)) = parser::load_map(diff.path.clone(), &stream_handle, key_mode).await {
                            state = Some(s);
                            
                            // Store the sink so we can pause/resume it
                            if let Ok(mut sink_lock) = audio_sink.lock() {
                                *sink_lock = Some(sink);
                            }
                            
                            scene = "Playing";
                            song_finished_shown = false;
                            let map_name = selected_beatmap.as_ref().unwrap().file_stem().unwrap().to_string_lossy();
                            rpc.update_playing(&map_name, &diff.version);
                        }
                    }
                }
            }
            "Playing" => {
                if let Some(ref mut s) = state {
                    let should_quit = game::update_and_draw(s, &mut options);
                    
                    if should_quit {
                        // Stop audio
                        if let Ok(mut sink) = audio_sink.lock() {
                            if let Some(sink_inst) = sink.take() {
                                sink_inst.stop();
                            }
                        }
                        scene = "DiffSelect";
                        rpc.update_idle();
                    }
                    
                    // Check if song finished
                    if s.song_finished && !song_finished_shown {
                        if let Ok(mut sink) = audio_sink.lock() {
                            if let Some(sink_inst) = sink.take() {
                                sink_inst.stop();
                            }
                        }
                        rpc.update_finished(&s.song_name);
                        song_finished_shown = true;
                    }
                }
            }
            _ => {}
        }
        next_frame().await
    }
}

fn get_pressed_key() -> Option<KeyCode> {
    // Check all common keys
    let keys = [
        KeyCode::A, KeyCode::B, KeyCode::C, KeyCode::D, KeyCode::E, KeyCode::F, KeyCode::G, KeyCode::H,
        KeyCode::I, KeyCode::J, KeyCode::K, KeyCode::L, KeyCode::M, KeyCode::N, KeyCode::O, KeyCode::P,
        KeyCode::Q, KeyCode::R, KeyCode::S, KeyCode::T, KeyCode::U, KeyCode::V, KeyCode::W, KeyCode::X,
        KeyCode::Y, KeyCode::Z,
        KeyCode::Key0, KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
        KeyCode::Key5, KeyCode::Key6, KeyCode::Key7, KeyCode::Key8, KeyCode::Key9,
        KeyCode::Space, KeyCode::LeftShift, KeyCode::RightShift, KeyCode::LeftControl, KeyCode::RightControl,
        KeyCode::LeftAlt, KeyCode::RightAlt, KeyCode::Comma, KeyCode::Period, KeyCode::Slash,
        KeyCode::Semicolon, KeyCode::Apostrophe, KeyCode::LeftBracket, KeyCode::RightBracket,
    ];
    
    for key in keys.iter() {
        if is_key_pressed(*key) {
            return Some(*key);
        }
    }
    
    None
}