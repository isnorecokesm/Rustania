#![windows_subsystem = "windows"]

mod models;
mod parser;
mod game;
mod discord_rpc;

use macroquad::prelude::*;
use macroquad::ui::root_ui;
use rodio::{OutputStream, Sink};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[macroquad::main("Rustania")]
async fn main() {
    let mut scene = "Menu";
    let mut state: Option<models::GameState> = None;
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let audio_sink: Arc<Mutex<Option<Sink>>> = Arc::new(Mutex::new(None));
    let mut selected_beatmap: Option<PathBuf> = None;
    let mut difficulties: Vec<parser::BeatmapInfo> = Vec::new();
    let mut key_mode = 4; // 2K or 4K
    let mut rpc = discord_rpc::RpcManager::new();
    rpc.update_idle();
    let _ = fs::create_dir_all("beatmaps");

    loop {
        clear_background(BLACK);
        match scene {
            "Menu" => {
                draw_rectangle(0.0, 0.0, 500.0, screen_height(), Color::new(0.1, 0.1, 0.1, 1.0));
                draw_text("RUSANIA", 40.0, 60.0, 40.0, SKYBLUE);
                
                // Key mode selector
                draw_text(&format!("MODE: {}K", key_mode), 40.0, 110.0, 25.0, WHITE);
                if root_ui().button(vec2(40.0, 125.0), "2K") {
                    key_mode = 2;
                }
                if root_ui().button(vec2(100.0, 125.0), "4K") {
                    key_mode = 4;
                }
                
                if root_ui().button(vec2(40.0, 180.0), "IMPORT .OSZ FILE") {
                    if let Some(path) = rfd::FileDialog::new().add_filter("osu", &["osz"]).pick_file() {
                        let _ = parser::import_osz(path);
                    }
                }

                draw_text("BEATMAPS:", 40.0, 250.0, 25.0, GRAY);
                
                if let Ok(entries) = fs::read_dir("beatmaps") {
                    for (i, entry) in entries.flatten().enumerate() {
                        let name = entry.file_name().into_string().unwrap_or_default();
                        let truncated = if name.len() > 35 {
                            format!("{}...", &name[..32])
                        } else {
                            name.clone()
                        };
                        
                        if root_ui().button(vec2(40.0, 280.0 + (i as f32 * 40.0)), truncated.as_str()) {
                            selected_beatmap = Some(entry.path());
                            if let Ok(diffs) = parser::get_difficulties(&entry.path()) {
                                difficulties = diffs;
                                scene = "DiffSelect";
                            }
                        }
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
                }
                
                if root_ui().button(vec2(40.0, 145.0), "< BACK TO MENU") {
                    scene = "Menu";
                    rpc.update_idle();
                }

                for (i, diff) in difficulties.iter().enumerate() {
                    if root_ui().button(vec2(40.0, 195.0 + (i as f32 * 40.0)), diff.version.as_str()) {
                        if let Ok(s) = parser::load_map(diff.path.clone(), &stream_handle, key_mode).await {
                            state = Some(s);
                            scene = "Playing";
                                  let map_name = selected_beatmap.as_ref().unwrap().file_stem().unwrap().to_string_lossy();
rpc.update_playing(&map_name, &diff.version);
                        }
                    }
                }
            }
            "Playing" => {
                if let Some(ref mut s) = state {
                    game::update_and_draw(s);
                    
                    // Check if song finished, stop audio
                    if s.song_finished {
                        if let Ok(mut sink) = audio_sink.lock() {
                            if let Some(s) = sink.take() {
                                s.stop();
                                
                            }
                        }
                    }
                    
                    if is_key_pressed(KeyCode::Escape) { 
                        // Stop audio when leaving
                        if let Ok(mut sink) = audio_sink.lock() {
                            if let Some(s) = sink.take() {
                                s.stop();
                            }
                        }
                        scene = "DiffSelect"; 
                    }
                }
            }
            _ => {}
        }
        next_frame().await
    }
}