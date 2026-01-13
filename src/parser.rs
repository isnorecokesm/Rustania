use crate::models::{Note, GameState, HitCounts};
use macroquad::prelude::*;
use rodio::{buffer::SamplesBuffer, OutputStreamHandle, Decoder, Source};
use std::fs;
use std::path::{Path, PathBuf};
use std::io::{BufReader, Read};
use std::time::Instant;

pub fn import_osz(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let target_dir = Path::new("beatmaps").join(path.file_stem().unwrap());
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)?;
        let file = fs::File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(&target_dir)?;
    }
    Ok(())
}

pub struct BeatmapInfo {
    pub path: PathBuf,
    pub version: String,
}

pub fn get_difficulties(folder_path: &PathBuf) -> Result<Vec<BeatmapInfo>, Box<dyn std::error::Error>> {
    let mut beatmaps = Vec::new();
    
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("osu") {
            let mut content = String::new();
            fs::File::open(&path)?.read_to_string(&mut content)?;
            
            let mut version = String::from("Unknown");
            
            for line in content.lines() {
                let line = line.trim();
                if line.starts_with("Version:") {
                    version = line.split(':').nth(1).unwrap_or("Unknown").trim().to_string();
                    break;
                }
            }
            
            beatmaps.push(BeatmapInfo { path, version });
        }
    }
    
    Ok(beatmaps)
}

pub async fn load_map(osu_path: PathBuf, stream: &OutputStreamHandle, force_key_count: usize) -> Result<GameState, Box<dyn std::error::Error>> {
    let mut osu_content = String::new();
    fs::File::open(&osu_path)?.read_to_string(&mut osu_content)?;

    let mut audio_filename = String::new();
    let mut slider_multiplier = 1.4;
    // Find background image
let mut bg_texture: Option<Texture2D> = None;
let mut section = "";
for line in osu_content.lines() {
    let line = line.trim();
    if line.starts_with("[") { section = line; continue; }

    if section == "[Events]" && line.starts_with("0,0,\"") {
        let start = line.find('"').unwrap() + 1;
        let end = line[start..].find('"').unwrap() + start;
        let bg_file = line[start..end].to_string();
        let bg_path = osu_path.parent().unwrap().join(bg_file);

        // Load texture
        if let Ok(tex) = macroquad::prelude::load_texture(bg_path.to_str().unwrap()).await {
            bg_texture = Some(tex);
        }
        break;
    }
}

    // Timing points: (time_ms, beat_length, velocity_multiplier)
    let mut timing_points: Vec<(f32, f32, f32)> = Vec::new();

    let mut section = "";
    for line in osu_content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") { continue; }
        if line.starts_with("[") { section = line; continue; }
        
        match section {
            "[General]" => if line.starts_with("AudioFilename:") {
                audio_filename = line.split(':').nth(1).unwrap().trim().to_string();
            },
            "[Difficulty]" => {
                if line.starts_with("SliderMultiplier:") {
                    slider_multiplier = line.split(':').nth(1).unwrap().trim().parse().unwrap_or(1.4);
                }
            },
            "[TimingPoints]" => {
                let p: Vec<&str> = line.split(',').collect();
                if p.len() >= 2 {
                    let time: f32 = p[0].parse().unwrap_or(0.0);
                    let val: f32 = p[1].parse().unwrap_or(500.0);
                    
                    if val > 0.0 {
                        // Uninherited timing point (red line) - sets BPM
                        timing_points.push((time, val, 1.0));
                    } else {
                        // Inherited timing point (green line) - velocity multiplier
                        // -100 = 1x, -50 = 2x, -200 = 0.5x
                        let velocity_mult = -100.0 / val;
                        let beat_length = timing_points.iter()
                            .rev()
                            .find(|(_, bl, _)| *bl > 0.0)
                            .map(|(_, bl, _)| *bl)
                            .unwrap_or(500.0);
                        timing_points.push((time, beat_length, velocity_mult));
                    }
                }
            },
            _ => {}
        }
    }
    
    if timing_points.is_empty() {
        timing_points.push((0.0, 500.0, 1.0));
    }

    // Load audio
    let folder_path = osu_path.parent().unwrap();
    let audio_path = folder_path.join(audio_filename);
    let source = Decoder::new(BufReader::new(fs::File::open(audio_path)?))?;
    let (sr, ch) = (source.sample_rate(), source.channels());
    let samples: Vec<f32> = source.convert_samples().collect();
    
    // Calculate song duration
    let song_duration = samples.len() as f32 / (sr as f32 * ch as f32);
    
    stream.play_raw(SamplesBuffer::new(ch, sr, samples))?;

    let mut notes = Vec::new();
    let mut in_hit_objects = false;

    for line in osu_content.lines() {
        if line.contains("[HitObjects]") { in_hit_objects = true; continue; }
        if in_hit_objects && !line.trim().is_empty() {
            let p: Vec<&str> = line.split(',').collect();
            if p.len() < 4 { continue; }

            let x: f32 = p[0].parse().unwrap_or(0.0);
            let time_ms: f32 = p[2].parse().unwrap_or(0.0);
            let obj_type: i32 = p[3].parse().unwrap_or(0);
            
            // Calculate lane based on x position
            // osu!mania uses 512px playfield width
            let lane = ((x * force_key_count as f32) / 512.0).floor() as usize;
            let lane = lane.clamp(0, force_key_count - 1);

            let mut end_time_ms = 0.0;
            
            // Bit flags: 1=circle, 2=slider, 8=spinner, 128=hold
            let is_hold = (obj_type & 128) != 0;
            let is_slider = (obj_type & 2) != 0;

            if is_hold && p.len() > 5 {
                // Hold note: x,y,time,type,hitSound,endTime:hitSample
                let end_str = p[5].split(':').next().unwrap_or("0");
                end_time_ms = end_str.parse::<f32>().unwrap_or(0.0);
            } else if is_slider && p.len() >= 8 {
                // Slider: x,y,time,type,hitSound,curveType|points,slides,length
                let slides: f32 = p[6].parse().unwrap_or(1.0); // repeat count
                let pixel_length: f32 = p[7].parse().unwrap_or(0.0);
                
                // Find active timing point
                let (beat_length, velocity_mult) = timing_points.iter()
                    .rev()
                    .find(|(time, _, _)| *time <= time_ms)
                    .map(|(_, bl, vm)| (*bl, *vm))
                    .unwrap_or((500.0, 1.0));
                
                // Formula: duration = (length / (SliderMultiplier * 100 * SV)) * beatLength * slides
                let base_velocity = slider_multiplier * 100.0 * velocity_mult;
                let duration_ms = (pixel_length / base_velocity) * beat_length * slides;
                end_time_ms = time_ms + duration_ms;
            }

            notes.push(Note { 
                start_time: time_ms / 1000.0, 
                end_time: end_time_ms / 1000.0, 
                lane, 
                hit: false, 
                missed: false,
                ln_started: false,
                ln_completed: false,
            });
        }
    }

    Ok(GameState {
        notes, 
        score: 0, 
        combo: 0, 
        last_judgment: "", 
        judgment_color: WHITE, 
        judgment_time: -1.0, 
        start_time: Instant::now(), 
        key_count: force_key_count,
        last_input_delay: 0.0,
        hit_counts: HitCounts {
            perfect: 0,
            great: 0,
            good: 0,
            ok: 0,
            miss: 0,
        },
        song_finished: false,
        song_duration,
        bg_texture,
    })
}