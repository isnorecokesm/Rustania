use macroquad::prelude::*;
use crate::models::{GameState, HitJudgment, GameOptions};

// osu!mania timing windows (in seconds)
const OK_WINDOW: f32 = 0.135;       // ±135ms = 50

pub fn update_and_draw(state: &mut GameState, options: &mut GameOptions) -> bool {
    let dt = get_frame_time();
    
    // Handle scroll speed changes with F3/F4
    if is_key_pressed(KeyCode::F3) && !state.paused && !state.song_finished {
        options.scroll_speed = (options.scroll_speed + 1).min(40); // F3 = FASTER (increase)
        state.speed_change_time = state.start_time.elapsed().as_secs_f32() - state.total_pause_time;
        state.speed_display_text = format!("osu!mania speed set to {}", options.scroll_speed);
        let _ = options.save(); // Auto-save on change
    }
    
    if is_key_pressed(KeyCode::F4) && !state.paused && !state.song_finished {
        options.scroll_speed = (options.scroll_speed - 1).max(1); // F4 = SLOWER (decrease)
        state.speed_change_time = state.start_time.elapsed().as_secs_f32() - state.total_pause_time;
        state.speed_display_text = format!("osu!mania speed set to {}", options.scroll_speed);
        let _ = options.save(); // Auto-save on change
    }
    
    // F5 resets to default (20)
    if is_key_pressed(KeyCode::F5) && !state.paused && !state.song_finished {
        options.scroll_speed = 20;
        state.speed_change_time = state.start_time.elapsed().as_secs_f32() - state.total_pause_time;
        state.speed_display_text = format!("osu!mania speed reset to {}", options.scroll_speed);
        let _ = options.save(); // Auto-save on change
    }
    
    // Handle pause - but NOT on results screen
    if is_key_pressed(KeyCode::Escape) && !state.song_finished {
        state.paused = !state.paused;
        if state.paused {
            state.pause_start = Some(std::time::Instant::now());
        } else {
            if let Some(pause_start) = state.pause_start {
                state.total_pause_time += pause_start.elapsed().as_secs_f32();
            }
            state.pause_start = None;
        }
    }
    
    if state.paused {
        // Stop all slider sounds when paused
        if let Some(audio) = &state.audio {
            audio.stop_all_sliders();
        }
        draw_pause_menu(state, options);
        return false; // Don't quit
    }
    
    let now = state.start_time.elapsed().as_secs_f32() - state.total_pause_time;
    
    if now >= state.song_duration + 2.0 {
        state.song_finished = true;
        // Stop all slider sounds when song finishes
        if let Some(audio) = &state.audio {
            audio.stop_all_sliders();
        }
    }
    
    if state.song_finished {
        draw_results_screen(state);
        // Allow returning to song select
        if is_key_pressed(KeyCode::Escape) {
            return true; // Signal to quit to song select
        }
        return false; // Don't quit yet
    }
    
    // Calculate scroll speed based on setting (1-40)
    // Formula: higher number = faster scroll
    let scroll_speed = 400.0 + (options.scroll_speed as f32 * 50.0);
    
    let lane_w = 100.0; 
    let total_w = lane_w * state.key_count as f32;
    let start_x = (screen_width() - total_w) / 2.0;
    let playfield_height = screen_height();
    
    // FNF MODE: Hit zone at TOP, notes scroll DOWN (spawn at bottom)
    // NORMAL MODE: Hit zone at BOTTOM, notes scroll UP (spawn at top)
    let hit_zone = if options.reverse_mode {
        playfield_height * 0.15  // Top of screen for FNF
    } else {
        playfield_height * 0.85  // Bottom of screen for normal
    };
    
    // Draw background
    if let Some(bg) = &state.bg_texture {
        draw_texture_ex(
            bg, 0.0, 0.0, WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            }
        );
    } else {
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), BLACK);
    }
    
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.4));
    draw_rectangle(start_x, hit_zone, total_w, 4.0, WHITE);
    
    for i in 1..state.key_count {
        let lx = start_x + (i as f32 * lane_w);
        draw_line(lx, 0.0, lx, playfield_height, 1.0, Color::new(0.3, 0.3, 0.3, 0.5));
    }

    let keys = if state.key_count == 2 { 
        &options.keys_2k[..]
    } else { 
        &options.keys_4k[..]
    };

    // Draw lane highlights and labels
    for (i, key) in keys.iter().enumerate() {
        let lx = start_x + (i as f32 * lane_w);

        if is_key_down(*key) {
            draw_rectangle(lx, 0.0, lane_w, playfield_height, Color::new(1.0, 1.0, 1.0, 0.1));
        }

        let label = format!("{:?}", key);
        let measure = measure_text(&label, None, 30, 1.0);
        let label_x = lx + (lane_w - measure.width) / 2.0;
        let label_y = if options.reverse_mode {
            hit_zone + 40.0  // Below hit zone for FNF
        } else {
            hit_zone - 10.0  // Above hit zone for normal
        };
        draw_text(&label, label_x, label_y, 30.0, WHITE);
    }

    // === KEY PRESS HANDLING ===
    for (i, key) in keys.iter().enumerate() {
        if is_key_pressed(*key) {
            handle_key_press(state, i, now);
        }
    }

    // === KEY RELEASE HANDLING (for LN tails) ===
    for (i, key) in keys.iter().enumerate() {
        if is_key_released(*key) {
            handle_key_release(state, i, now);
        }
    }

    // === HOLD INTEGRITY CHECKING ===
    for (i, key) in keys.iter().enumerate() {
        let is_holding = is_key_down(*key);
        check_ln_hold_integrity(state, i, is_holding, now);
    }
    
    // === SLIDER SOUND MANAGEMENT ===
    // Start/stop slider sounds based on hold state
    for note in state.notes.iter_mut() {
        if !note.is_ln { continue; }
        if !note.ln_head_hit || note.ln_completed || note.ln_hold_broken { 
            // Stop sound if it was playing
            if note.slider_sound_playing {
                if let Some(audio) = &state.audio {
                    audio.stop_slider(note.lane);
                }
                note.slider_sound_playing = false;
            }
            continue;
        }
        
        // Check if we should be playing the slider sound
        if now >= note.start_time && now < note.end_time {
            if !note.slider_sound_playing {
                // Start slider sound
                if let Some(audio) = &state.audio {
                    audio.play_slider_start(note.lane);
                }
                note.slider_sound_playing = true;
            }
        }
    }

    // === MISS CHECKING ===
    check_missed_notes(state, now);

    // === DRAWING NOTES ===
    for note in state.notes.iter() {
        // Don't draw completely missed regular notes
        if note.missed && !note.is_ln {
            continue;
        }
        
        let x = start_x + (note.lane as f32 * lane_w);
        
        // Calculate Y position based on mode
        let y_head = if options.reverse_mode {
            // FNF: notes scroll DOWN (positive direction)
            hit_zone + ((note.start_time - now) * scroll_speed)
        } else {
            // Normal: notes scroll UP (negative direction)
            hit_zone - ((note.start_time - now) * scroll_speed)
        };

        if note.is_ln {
            let y_tail = if options.reverse_mode {
                hit_zone + ((note.end_time - now) * scroll_speed)
            } else {
                hit_zone - ((note.end_time - now) * scroll_speed)
            };
            
            // Check visibility based on mode
            let is_visible = if options.reverse_mode {
                y_head < screen_height() && y_tail > -1000.0
            } else {
                y_tail < screen_height() && y_head > -1000.0
            };
            
            if is_visible {
                let color = if note.ln_completed {
                    Color::new(0.3, 0.8, 0.3, 0.6) // Green - completed
                } else if note.ln_hold_broken || note.missed {
                    Color::new(0.8, 0.2, 0.2, 0.6) // Red - broken/missed
                } else if note.ln_head_hit && now >= note.start_time && now < note.end_time {
                    SKYBLUE // Blue - actively holding
                } else {
                    Color::new(0.4, 0.4, 0.4, 0.6) // Gray - upcoming
                };
                
                // Draw LN body
                let top = if note.ln_head_hit && now >= note.start_time {
                    hit_zone
                } else {
                    y_head
                };

                let bottom = if note.ln_completed || (note.ln_hold_broken && now >= note.end_time) {
                    hit_zone
                } else {
                    y_tail
                };
                
                let rect_y = top.min(bottom);
                let rect_height = (top - bottom).abs().max(2.0);
                draw_rectangle(x + 10.0, rect_y, lane_w - 20.0, rect_height, color);
            }
            
            // Draw head if not yet hit
            let head_visible = if options.reverse_mode {
                y_head > -50.0 && y_head < screen_height()
            } else {
                y_head < screen_height() && y_head > -50.0
            };
            
            if !note.ln_head_hit && !note.missed && head_visible {
                draw_rectangle(x + 4.0, y_head - 15.0, lane_w - 8.0, 30.0, WHITE);
            }
        } else {
            // Regular note
            let note_visible = if options.reverse_mode {
                y_head > -50.0 && y_head < screen_height()
            } else {
                y_head < screen_height() && y_head > -50.0
            };
            
            if !note.hit && note_visible {
                draw_rectangle(x + 4.0, y_head - 15.0, lane_w - 8.0, 30.0, WHITE);
            }
        }
    }

    // === UI ELEMENTS ===
    let combo_text = format!("{}", state.combo);
    let measure = measure_text(&combo_text, None, 60, 1.0);
    draw_text(&combo_text, (screen_width() - measure.width)/2.0, 200.0, 60.0, WHITE);
    
    if now - state.judgment_time < 0.5 {
        let jtext = state.last_judgment;
        let jmeasure = measure_text(jtext, None, 40, 1.0);
        let jx = (screen_width() - jmeasure.width) / 2.0;
        
        let jcolor = if jtext == "PERFECT" {
            let hue = (now * 3.0) % 1.0;
            let rgb = hsv_to_rgb(hue, 0.8, 1.0);
            Color::new(rgb.0, rgb.1, rgb.2, 1.0)
        } else {
            state.judgment_color
        };
        
        draw_text(jtext, jx, 280.0, 40.0, jcolor);
    }
    
    // Show scroll speed change notification
    if now - state.speed_change_time < 2.0 {
        let alpha = if now - state.speed_change_time < 1.5 {
            1.0
        } else {
            1.0 - ((now - state.speed_change_time - 1.5) / 0.5)
        };
        
        let speed_measure = measure_text(&state.speed_display_text, None, 25, 1.0);
        let speed_x = (screen_width() - speed_measure.width) / 2.0;
        draw_text(&state.speed_display_text, speed_x, 350.0, 25.0, 
                  Color::new(1.0, 1.0, 1.0, alpha));
    }
    
    // Debug info
    draw_text(&format!("FPS: {}", get_fps()), 10.0, 25.0, 20.0, GREEN);
    
    let frame_ms = dt * 1000.0;
    let frame_color = if frame_ms < 5.0 { GREEN } else if frame_ms < 10.0 { YELLOW } else { RED };
    draw_text(&format!("Frame: {:.1}ms", frame_ms), 10.0, 50.0, 20.0, frame_color);
    
    let delay_color = if state.last_input_delay.abs() < 10.0 { GREEN } 
                      else if state.last_input_delay.abs() < 30.0 { YELLOW } 
                      else { RED };
    draw_text(&format!("Timing: {:.1}ms", state.last_input_delay), 10.0, 75.0, 20.0, delay_color);
    draw_text(&format!("Score: {}", state.score), 10.0, 100.0, 20.0, WHITE);
    draw_text(&format!("Speed: {} (F3/F4)", options.scroll_speed), 10.0, 125.0, 20.0, SKYBLUE);
    
    false // Don't quit
}

fn handle_key_press(state: &mut GameState, lane: usize, now: f32) {
    // Find closest unhit note in this lane
    let mut closest_note: Option<(usize, f32)> = None;

    for (idx, note) in state.notes.iter().enumerate() {
        if note.lane != lane { continue; }
        
        // For LNs: only consider if head not yet hit
        // For regular notes: only consider if not hit
        let can_hit = if note.is_ln {
            !note.ln_head_hit && !note.missed
        } else {
            !note.hit && !note.missed
        };
        
        if !can_hit { continue; }
        
        let time_diff = (note.start_time - now).abs();
        if time_diff < OK_WINDOW {
            if let Some((_, current_diff)) = closest_note {
                if time_diff < current_diff {
                    closest_note = Some((idx, time_diff));
                }
            } else {
                closest_note = Some((idx, time_diff));
            }
        }
    }

    if let Some((idx, _)) = closest_note {
        let note = &mut state.notes[idx];
        let timing_diff = note.start_time - now;
        let abs_timing = timing_diff.abs();
        
        let judgment = HitJudgment::from_timing(abs_timing);
        
        if judgment != HitJudgment::Miss {
            if note.is_ln {
                // LN HEAD HIT
                note.ln_head_hit = true;
                note.ln_head_judgment = Some(judgment);
                state.combo += 1;
                state.score += judgment.score_value();
                
                // Update hit counts for head
                match judgment {
                    HitJudgment::Perfect => state.hit_counts.perfect += 1,
                    HitJudgment::Great => state.hit_counts.great += 1,
                    HitJudgment::Good => state.hit_counts.good += 1,
                    HitJudgment::Ok => state.hit_counts.ok += 1,
                    _ => {}
                }
                
                state.last_judgment = judgment.text();
                state.judgment_color = judgment.color();
                state.judgment_time = now;
                state.last_input_delay = timing_diff * 1000.0;
                
                // Play hit sound
                if let Some(audio) = &state.audio {
                    audio.play_hit();
                }
            } else {
                // REGULAR NOTE HIT
                note.hit = true;
                state.combo += 1;
                state.score += judgment.score_value();
                
                match judgment {
                    HitJudgment::Perfect => state.hit_counts.perfect += 1,
                    HitJudgment::Great => state.hit_counts.great += 1,
                    HitJudgment::Good => state.hit_counts.good += 1,
                    HitJudgment::Ok => state.hit_counts.ok += 1,
                    _ => {}
                }
                
                state.last_judgment = judgment.text();
                state.judgment_color = judgment.color();
                state.judgment_time = now;
                state.last_input_delay = timing_diff * 1000.0;
                
                // Play hit sound
                if let Some(audio) = &state.audio {
                    audio.play_hit();
                }
            }
        }
    }
}

fn handle_key_release(state: &mut GameState, lane: usize, now: f32) {
    // Find active LNs in this lane that are waiting for release
    for note in state.notes.iter_mut() {
        if note.lane != lane { continue; }
        if !note.is_ln { continue; }
        if !note.ln_head_hit { continue; }
        if note.ln_completed || note.ln_hold_broken { continue; }
        
        // Check if we're within the tail's timing window
        let tail_timing_diff = note.end_time - now;
        let abs_tail_timing = tail_timing_diff.abs();
        
        if abs_tail_timing < OK_WINDOW {
            // Valid tail release
            let tail_judgment = HitJudgment::from_timing(abs_tail_timing);
            
            note.ln_tail_judgment = Some(tail_judgment);
            note.ln_completed = true;
            state.combo += 1;
            state.score += tail_judgment.score_value();
            
            // Update hit counts for tail
            match tail_judgment {
                HitJudgment::Perfect => state.hit_counts.perfect += 1,
                HitJudgment::Great => state.hit_counts.great += 1,
                HitJudgment::Good => state.hit_counts.good += 1,
                HitJudgment::Ok => state.hit_counts.ok += 1,
                _ => {}
            }
            
            state.last_judgment = tail_judgment.text();
            state.judgment_color = tail_judgment.color();
            state.judgment_time = now;
            state.last_input_delay = tail_timing_diff * 1000.0;
            
            // Play hit sound for tail
            if let Some(audio) = &state.audio {
                audio.play_hit();
            }
            
            // Only process one LN tail per release
            break;
        }
    }
}

fn check_ln_hold_integrity(state: &mut GameState, lane: usize, is_holding: bool, now: f32) {
    for note in state.notes.iter_mut() {
        if note.lane != lane { continue; }
        if !note.is_ln { continue; }
        if !note.ln_head_hit { continue; }
        if note.ln_completed || note.ln_hold_broken { continue; }
        
        // Check if we're in the hold phase
        if now >= note.start_time && now < note.end_time {
            if !is_holding {
                // HOLD BROKEN - released too early
                note.ln_hold_broken = true;
                note.ln_completed = true;
                note.ln_tail_judgment = Some(HitJudgment::Miss);
                state.combo = 0;
                state.last_judgment = "MISS";
                state.judgment_color = RED;
                state.judgment_time = now;
                state.hit_counts.miss += 1;
            }
        }
    }
}

fn check_missed_notes(state: &mut GameState, now: f32) {
    for note in state.notes.iter_mut() {
        if note.missed { continue; }
        
        if note.is_ln {
            // Check if LN head was missed
            if !note.ln_head_hit && (note.start_time - now) < -OK_WINDOW {
                note.missed = true;
                note.ln_completed = true;
                note.ln_head_judgment = Some(HitJudgment::Miss);
                note.ln_tail_judgment = Some(HitJudgment::Miss);
                state.combo = 0;
                state.last_judgment = "MISS";
                state.judgment_color = RED;
                state.judgment_time = now;
                state.hit_counts.miss += 2; // Both head and tail missed
            }
            // Check if LN tail was missed (head was hit but never released)
            else if note.ln_head_hit && !note.ln_completed && (note.end_time - now) < -OK_WINDOW {
                note.missed = true;
                note.ln_completed = true;
                note.ln_tail_judgment = Some(HitJudgment::Miss);
                state.combo = 0;
                state.last_judgment = "MISS";
                state.judgment_color = RED;
                state.judgment_time = now;
                state.hit_counts.miss += 1; // Just tail missed
            }
        } else {
            // Regular note missed
            if !note.hit && (note.start_time - now) < -OK_WINDOW {
                note.missed = true;
                state.combo = 0;
                state.last_judgment = "MISS";
                state.judgment_color = RED;
                state.judgment_time = now;
                state.hit_counts.miss += 1;
            }
        }
    }
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h_prime = (h * 6.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h_prime < 1.0 { (c, x, 0.0) }
    else if h_prime < 2.0 { (x, c, 0.0) }
    else if h_prime < 3.0 { (0.0, c, x) }
    else if h_prime < 4.0 { (0.0, x, c) }
    else if h_prime < 5.0 { (x, 0.0, c) }
    else { (c, 0.0, x) };
    
    (r + m, g + m, b + m)
}

fn draw_pause_menu(state: &GameState, options: &GameOptions) {
    // Darken background
    draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.0, 0.7));
    
    let cx = screen_width() / 2.0;
    let cy = screen_height() / 2.0;
    
    // Pause text
    let pause_text = "PAUSED";
    let measure = measure_text(pause_text, None, 80, 1.0);
    draw_text(pause_text, cx - measure.width / 2.0, cy - 80.0, 80.0, WHITE);
    
    // Current stats
    let combo_text = format!("Combo: {}", state.combo);
    let combo_measure = measure_text(&combo_text, None, 30, 1.0);
    draw_text(&combo_text, cx - combo_measure.width / 2.0, cy - 20.0, 30.0, GRAY);
    
    let score_text = format!("Score: {}", state.score);
    let score_measure = measure_text(&score_text, None, 30, 1.0);
    draw_text(&score_text, cx - score_measure.width / 2.0, cy + 20.0, 30.0, GRAY);
    
    let speed_text = format!("Scroll Speed: {}", options.scroll_speed);
    let speed_measure = measure_text(&speed_text, None, 25, 1.0);
    draw_text(&speed_text, cx - speed_measure.width / 2.0, cy + 60.0, 25.0, SKYBLUE);
    
    // Instructions
    let resume_text = "Press ESC to Resume";
    let resume_measure = measure_text(resume_text, None, 25, 1.0);
    draw_text(resume_text, cx - resume_measure.width / 2.0, cy + 110.0, 25.0, YELLOW);
}

fn draw_results_screen(state: &GameState) {
    clear_background(BLACK);
    
    let total_objects = (state.hit_counts.perfect + state.hit_counts.great + 
                        state.hit_counts.good + state.hit_counts.ok + 
                        state.hit_counts.miss) as f32;
    
    let accuracy = if total_objects > 0.0 {
        let weighted_score = (300.0 * state.hit_counts.perfect as f32) +
                            (200.0 * state.hit_counts.great as f32) +
                            (100.0 * state.hit_counts.good as f32) +
                            (50.0 * state.hit_counts.ok as f32);
        let max_score = 300.0 * total_objects;
        (weighted_score / max_score) * 100.0
    } else {
        0.0
    };
    
    let (grade, grade_color) = if accuracy >= 100.0 && state.hit_counts.miss == 0 {
        ("SS", Color::new(1.0, 0.9, 0.0, 1.0))
    } else if accuracy >= 95.0 && state.hit_counts.miss == 0 {
        ("S", Color::new(0.9, 0.9, 0.9, 1.0))
    } else if accuracy >= 90.0 {
        ("A", Color::new(0.2, 1.0, 0.3, 1.0))
    } else if accuracy >= 80.0 {
        ("B", Color::new(0.3, 0.8, 1.0, 1.0))
    } else if accuracy >= 70.0 {
        ("C", Color::new(0.9, 0.6, 0.2, 1.0))
    } else {
        ("D", Color::new(0.9, 0.3, 0.3, 1.0))
    };
    
    let cx = screen_width() / 2.0;
    
    draw_text("RESULTS", cx - 100.0, 80.0, 50.0, WHITE);
    
    let grade_size = 120.0;
    let grade_measure = measure_text(grade, None, grade_size as u16, 1.0);
    draw_text(grade, cx - grade_measure.width / 2.0, 200.0, grade_size, grade_color);
    
    let acc_text = format!("{:.2}%", accuracy);
    let acc_measure = measure_text(&acc_text, None, 40, 1.0);
    draw_text(&acc_text, cx - acc_measure.width / 2.0, 250.0, 40.0, WHITE);
    
    let score_text = format!("Score: {}", state.score);
    let score_measure = measure_text(&score_text, None, 35, 1.0);
    draw_text(&score_text, cx - score_measure.width / 2.0, 300.0, 35.0, GRAY);
    
    let y_start = 360.0;
    let spacing = 35.0;
    
    draw_text(&format!("PERFECT: {}", state.hit_counts.perfect), cx - 100.0, y_start, 25.0, Color::new(1.0, 0.8, 0.0, 1.0));
    draw_text(&format!("GREAT: {}", state.hit_counts.great), cx - 100.0, y_start + spacing, 25.0, Color::new(0.0, 1.0, 0.5, 1.0));
    draw_text(&format!("GOOD: {}", state.hit_counts.good), cx - 100.0, y_start + spacing * 2.0, 25.0, Color::new(0.3, 0.8, 1.0, 1.0));
    draw_text(&format!("OK: {}", state.hit_counts.ok), cx - 100.0, y_start + spacing * 3.0, 25.0, Color::new(0.7, 0.7, 0.7, 1.0));
    draw_text(&format!("MISS: {}", state.hit_counts.miss), cx - 100.0, y_start + spacing * 4.0, 25.0, RED);
    
    draw_text("Press ESC to return to song select", cx - 180.0, screen_height() - 40.0, 20.0, DARKGRAY);
}