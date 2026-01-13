use macroquad::prelude::*;
use crate::models::GameState;

const SCROLL_SPEED: f32 = 1500.0; 
const MIN_LN_DURATION: f32 = 0.15; // Minimum 150ms for long notes to be fair
const LN_END_LENIENCY: f32 = 0.08; // 80ms leniency for releasing at the end

// Timing windows (in seconds)
const PERFECT_WINDOW: f32 = 0.040;  // ±40ms
const GREAT_WINDOW: f32 = 0.075;    // ±75ms  
const GOOD_WINDOW: f32 = 0.110;     // ±110ms
const OK_WINDOW: f32 = 0.135;       // ±135ms

pub fn update_and_draw(state: &mut GameState) {
    let now = state.start_time.elapsed().as_secs_f32();
    let dt = get_frame_time();
    
    // Check if song is finished
    if now >= state.song_duration + 2.0 {
        state.song_finished = true;
    }
    
    // If song finished, show results screen
    if state.song_finished {
        draw_results_screen(state);
        return;
    }
    
    let lane_w = 100.0; 
    let total_w = lane_w * state.key_count as f32;
    
    // Center the playfield horizontally and position it better vertically
    let start_x = (screen_width() - total_w) / 2.0;
    let playfield_height = screen_height();
    let hit_zone = playfield_height * 0.85; // 85% down the screen
    
    // Draw playfield background
    draw_rectangle(start_x, 0.0, total_w, playfield_height, Color::new(0.0, 0.0, 0.0, 0.95));
    
    // Draw hit zone line
    draw_rectangle(start_x, hit_zone, total_w, 4.0, WHITE);
    
    // Draw lane separators
    for i in 1..state.key_count {
        let lx = start_x + (i as f32 * lane_w);
        draw_line(lx, 0.0, lx, playfield_height, 1.0, Color::new(0.3, 0.3, 0.3, 0.5));
    }

    let keys = if state.key_count == 2 { vec![KeyCode::D, KeyCode::K] } 
               else { vec![KeyCode::D, KeyCode::F, KeyCode::J, KeyCode::K] };

    // FIRST PASS: Handle key presses (start of notes)
    for (i, key) in keys.iter().enumerate() {
        let lx = start_x + (i as f32 * lane_w);
        if is_key_down(*key) {
            draw_rectangle(lx, 0.0, lane_w, hit_zone, Color::new(1.0, 1.0, 1.0, 0.1));
        }

        if is_key_pressed(*key) {
            // Find the closest unhit note in this lane within timing window
            let mut closest_note: Option<(usize, f32)> = None;
            
            for (idx, note) in state.notes.iter().enumerate() {
                if note.lane == i && !note.hit && !note.missed {
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
            }
            
            if let Some((idx, _)) = closest_note {
                let note = &mut state.notes[idx];
                let timing_diff = note.start_time - now;
                let abs_timing = timing_diff.abs();
                let duration = note.end_time - note.start_time;
                
                // Determine judgment based on timing
                let (judgment, judgment_color, score_val) = if abs_timing <= PERFECT_WINDOW {
                    state.hit_counts.perfect += 1;
                    ("PERFECT", Color::new(1.0, 0.8, 0.0, 1.0), 300) // Gold
                } else if abs_timing <= GREAT_WINDOW {
                    state.hit_counts.great += 1;
                    ("GREAT", Color::new(0.0, 1.0, 0.5, 1.0), 200) // Green
                } else if abs_timing <= GOOD_WINDOW {
                    state.hit_counts.good += 1;
                    ("GOOD", Color::new(0.3, 0.8, 1.0, 1.0), 100) // Blue
                } else {
                    state.hit_counts.ok += 1;
                    ("OK", Color::new(0.7, 0.7, 0.7, 1.0), 50) // Gray
                };
                
                // Check if it's a long note or regular note
                if duration >= MIN_LN_DURATION {
                    // Long note - mark head as hit
                    note.ln_started = true;
                    note.hit = true;
                    state.combo += 1;
                    state.score += score_val / 2; // Half score for head
                    state.last_judgment = judgment;
                    state.judgment_color = judgment_color;
                    state.judgment_time = now;
                    state.last_input_delay = timing_diff * 1000.0;
                } else {
                    // Regular note
                    note.hit = true;
                    note.missed = false;
                    state.combo += 1;
                    state.score += score_val;
                    state.last_judgment = judgment;
                    state.judgment_color = judgment_color;
                    state.judgment_time = now;
                    state.last_input_delay = timing_diff * 1000.0;
                }
            }
        }
    }

    // SECOND PASS: Handle long note holds and releases
    for (i, key) in keys.iter().enumerate() {
        let is_holding = is_key_down(*key);
        
        for note in state.notes.iter_mut().filter(|n| n.lane == i) {
            let duration = note.end_time - note.start_time;
            
            // Skip if not a long note
            if duration < MIN_LN_DURATION { continue; }
            
            // Skip if already processed
            if note.ln_completed { continue; }
            
            // Check if LN head was hit
            if note.ln_started && !note.missed {
                // Currently in the hold phase
                if now >= note.start_time && now < note.end_time {
                    // Check if key is still held
                    if !is_holding {
                        // Released too early - MISS
                        note.missed = true;
                        note.ln_completed = true; // Mark as processed
                        state.combo = 0;
                        state.last_judgment = "MISS";
                        state.judgment_color = RED;
                        state.judgment_time = now;
                        state.hit_counts.miss += 1;
                    }
                } else if now >= note.end_time && !note.ln_completed {
                    // Check completion - grant leniency window
                    let time_past_end = now - note.end_time;
                    
                    if time_past_end <= LN_END_LENIENCY {
                        // Still in leniency window
                        if is_holding {
                            // Still holding - success!
                            note.ln_completed = true;
                            state.score += 150; // Tail completion score
                        }
                    } else {
                        // Past leniency window - finalize
                        note.ln_completed = true;
                        if is_holding {
                            // Held through leniency - success
                            state.score += 150;
                        } else {
                            // Didn't hold - MISS the tail
                            note.missed = true;
                            state.combo = 0;
                            state.last_judgment = "MISS";
                            state.judgment_color = RED;
                            state.judgment_time = now;
                            state.hit_counts.miss += 1;
                        }
                    }
                }
            } else if !note.ln_started && !note.missed {
                // LN head was never hit, check if it should be marked as missed
                if (note.start_time - now) < -OK_WINDOW {
                    note.missed = true;
                    note.ln_completed = true; // Mark as processed to prevent re-checking
                    state.combo = 0;
                    state.last_judgment = "MISS";
                    state.judgment_color = RED;
                    state.judgment_time = now;
                    state.hit_counts.miss += 1;
                }
            }
        }
    }

    // THIRD PASS: Check for completely missed notes (never hit)
    for note in state.notes.iter_mut() {
        if note.hit || note.missed { continue; }
        
        // If past the timing window and not hit, it's a miss
        if (note.start_time - now) < -OK_WINDOW {
            note.missed = true;
            state.combo = 0;
            state.last_judgment = "MISS";
            state.judgment_color = RED;
            state.judgment_time = now;
            state.hit_counts.miss += 1;
        }
    }

    // DRAW NOTES
    for note in state.notes.iter() {
        // Don't draw regular notes that were missed
        if note.missed && note.end_time - note.start_time < MIN_LN_DURATION { 
            continue; 
        }
        
        let x = start_x + (note.lane as f32 * lane_w);
        let y_head = hit_zone - ((note.start_time - now) * SCROLL_SPEED);
        let duration = note.end_time - note.start_time;

        // Long note rendering
        if duration >= MIN_LN_DURATION {
            let y_tail = hit_zone - ((note.end_time - now) * SCROLL_SPEED);
            
            // Only draw if visible on screen
            if y_tail < screen_height() && y_head > -1000.0 {
                let color = if note.ln_completed {
                    Color::new(0.3, 0.8, 0.3, 0.6) // Green - completed successfully
                } else if note.missed {
                    Color::new(0.8, 0.2, 0.2, 0.6) // Red - missed/failed
                } else if note.ln_started && now >= note.start_time && now < note.end_time {
                    SKYBLUE // Blue - actively holding
                } else if note.ln_started && now >= note.end_time {
                    Color::new(0.5, 0.8, 0.5, 0.7) // Light green - waiting for completion check
                } else {
                    Color::new(0.4, 0.4, 0.4, 0.6) // Gray - upcoming
                };
                
                // Calculate body position
                let top = if note.ln_started && now >= note.start_time { 
                    hit_zone 
                } else { 
                    y_head 
                };
                
                let bottom = if note.ln_completed || (now >= note.end_time) { 
                    hit_zone 
                } else { 
                    y_tail 
                };
                
                // Draw LN body
                draw_rectangle(x + 10.0, bottom, lane_w - 20.0, (top - bottom).max(2.0), color);
            }
            
            // Draw head marker if not yet started
            if !note.ln_started && y_head < screen_height() && y_head > -50.0 {
                draw_rectangle(x + 4.0, y_head - 15.0, lane_w - 8.0, 30.0, YELLOW);
            }
        } else {
            // Regular note rendering
            if !note.hit && y_head < screen_height() && y_head > -50.0 {
                draw_rectangle(x + 4.0, y_head - 15.0, lane_w - 8.0, 30.0, WHITE);
            }
        }
    }

    // Draw combo (centered, upper area)
    let combo_text = format!("{}", state.combo);
    let measure = measure_text(&combo_text, None, 60, 1.0);
    draw_text(&combo_text, (screen_width() - measure.width)/2.0, 200.0, 60.0, WHITE);
    
    // Draw judgment (below combo)
    if now - state.judgment_time < 0.5 {
        let jtext = state.last_judgment;
        let jmeasure = measure_text(jtext, None, 40, 1.0);
        let jx = (screen_width() - jmeasure.width) / 2.0;
        
        // Rainbow effect for PERFECT
        let jcolor = if jtext == "PERFECT" {
            let hue = (now * 3.0) % 1.0;
            let rgb = hsv_to_rgb(hue, 0.8, 1.0);
            Color::new(rgb.0, rgb.1, rgb.2, 1.0)
        } else {
            state.judgment_color
        };
        
        draw_text(jtext, jx, 280.0, 40.0, jcolor);
    }
    
    // Draw debug info (top-left corner)
    let fps = get_fps();
    let debug_text = format!("FPS: {}", fps);
    draw_text(&debug_text, 10.0, 25.0, 20.0, GREEN);
    
    // Frame time (input delay from system)
    let frame_ms = dt * 1000.0;
    let frame_text = format!("Frame: {:.1}ms", frame_ms);
    let frame_color = if frame_ms < 5.0 {
        GREEN
    } else if frame_ms < 10.0 {
        YELLOW
    } else {
        RED
    };
    draw_text(&frame_text, 10.0, 50.0, 20.0, frame_color);
    
    // Player timing
    let delay_text = format!("Timing: {:.1}ms", state.last_input_delay);
    let delay_color = if state.last_input_delay.abs() < 10.0 {
        GREEN
    } else if state.last_input_delay.abs() < 30.0 {
        YELLOW
    } else {
        RED
    };
    draw_text(&delay_text, 10.0, 75.0, 20.0, delay_color);
    
    let score_text = format!("Score: {}", state.score);
    draw_text(&score_text, 10.0, 100.0, 20.0, WHITE);
}

// Helper function to convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h_prime = (h * 6.0) % 6.0;
    let x = c * (1.0 - ((h_prime % 2.0) - 1.0).abs());
    let m = v - c;
    
    let (r, g, b) = if h_prime < 1.0 {
        (c, x, 0.0)
    } else if h_prime < 2.0 {
        (x, c, 0.0)
    } else if h_prime < 3.0 {
        (0.0, c, x)
    } else if h_prime < 4.0 {
        (0.0, x, c)
    } else if h_prime < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    
    (r + m, g + m, b + m)
}

fn draw_results_screen(state: &GameState) {
    clear_background(BLACK);
    
    // Calculate accuracy
    let total_notes = state.notes.len() as f32;
    let total_hits = (state.hit_counts.perfect + state.hit_counts.great + 
                     state.hit_counts.good + state.hit_counts.ok) as f32;
    let accuracy = if total_notes > 0.0 {
        (total_hits / total_notes) * 100.0
    } else {
        0.0
    };
    
    // Determine grade
    let (grade, grade_color) = if accuracy >= 95.0 {
        ("SS", Color::new(1.0, 0.9, 0.0, 1.0)) // Gold
    } else if accuracy >= 90.0 {
        ("S", Color::new(1.0, 0.8, 0.2, 1.0)) // Light gold
    } else if accuracy >= 80.0 {
        ("A", Color::new(0.2, 1.0, 0.3, 1.0)) // Green
    } else if accuracy >= 70.0 {
        ("B", Color::new(0.3, 0.8, 1.0, 1.0)) // Blue
    } else if accuracy >= 60.0 {
        ("C", Color::new(0.9, 0.6, 0.2, 1.0)) // Orange
    } else {
        ("D", Color::new(0.9, 0.3, 0.3, 1.0)) // Red
    };
    
    let cx = screen_width() / 2.0;
    
    // Title
    draw_text("RESULTS", cx - 100.0, 80.0, 50.0, WHITE);
    
    // Grade (big and centered)
    let grade_size = 120.0;
    let grade_measure = measure_text(grade, None, grade_size as u16, 1.0);
    draw_text(grade, cx - grade_measure.width / 2.0, 200.0, grade_size, grade_color);
    
    // Accuracy
    let acc_text = format!("{:.2}%", accuracy);
    let acc_measure = measure_text(&acc_text, None, 40, 1.0);
    draw_text(&acc_text, cx - acc_measure.width / 2.0, 250.0, 40.0, WHITE);
    
    // Score
    let score_text = format!("Score: {}", state.score);
    let score_measure = measure_text(&score_text, None, 35, 1.0);
    draw_text(&score_text, cx - score_measure.width / 2.0, 300.0, 35.0, GRAY);
    
    // Hit counts
    let y_start = 360.0;
    let spacing = 35.0;
    
    draw_text(&format!("PERFECT: {}", state.hit_counts.perfect), cx - 100.0, y_start, 25.0, Color::new(1.0, 0.8, 0.0, 1.0));
    draw_text(&format!("GREAT: {}", state.hit_counts.great), cx - 100.0, y_start + spacing, 25.0, Color::new(0.0, 1.0, 0.5, 1.0));
    draw_text(&format!("GOOD: {}", state.hit_counts.good), cx - 100.0, y_start + spacing * 2.0, 25.0, Color::new(0.3, 0.8, 1.0, 1.0));
    draw_text(&format!("OK: {}", state.hit_counts.ok), cx - 100.0, y_start + spacing * 3.0, 25.0, Color::new(0.7, 0.7, 0.7, 1.0));
    draw_text(&format!("MISS: {}", state.hit_counts.miss), cx - 100.0, y_start + spacing * 4.0, 25.0, RED);
    
    // Instructions
    draw_text("Press ESC to return to menu", cx - 150.0, screen_height() - 40.0, 20.0, DARKGRAY);
}