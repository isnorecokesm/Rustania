# Rustania

## Project Overview

**Rustania** is a 2D rhythm game inspired by osu!mania, developed as a school project in Rust. Players hit notes in time with the music using the keyboard, aiming for high scores, combos, and accuracy.

This project is primarily educational and demonstrates parsing game files, input handling, audio playback, and simple game logic.

---

## Features

1. **2K and 4K Modes**

   * **2K mode:** 2 lanes, default keys `[D, K]`.
   * **4K mode:** 4 lanes, default keys `[D, F, J, K]`.
   * Players can select mode from the main menu.

2. **Beatmap Import**

   * Supports importing `.osz` files.
   * Extracts beatmaps into a `beatmaps/` folder.
   * Automatically parses `.osu` files to read note data, timing points, and slider multipliers.

3. **Gameplay Mechanics**

   * Notes scroll down to the **hit zone** at the bottom of the screen.
   * Supports **regular notes** and **long notes (LN)** with leniency at the tail end.
   * Timing windows for judgments:

     * **PERFECT:** ±40ms
     * **GREAT:** ±75ms
     * **GOOD:** ±110ms
     * **OK:** ±135ms
     * **MISS:** outside timing window
   * **Combo system** increases as notes are hit consecutively.
   * Long note handling:

     * Head must be hit within timing window.
     * Hold the key until the tail, with leniency of 80ms.
   * Judgments are color-coded:

     * PERFECT → rainbow effect
     * GREAT → green
     * GOOD → blue
     * OK → gray
     * MISS → red

4. **Results Screen**

   * Displays:

     * Total score
     * Accuracy (%)
     * Hit counts (PERFECT, GREAT, GOOD, OK, MISS)
     * Grade based on accuracy:

       * **SS:** ≥95%
       * **S:** ≥90%
       * **A:** ≥80%
       * **B:** ≥70%
       * **C:** ≥60%
       * **D:** <60%
   * Instruction to press ESC to return to menu

5. **Debug Features**

   * FPS display
   * Frame time (ms)
   * Player timing offset in milliseconds (early/late)

---

## Technical Details

### Language & Libraries

* **Rust** – language for the project.
* **Macroquad** – for rendering, input, UI, and game loop.
* **Rodio** – audio playback.
* **zip** – extraction of `.osz` files.

### File Structure

```
src/
├─ main.rs        # Main loop and menu
├─ game.rs        # Gameplay logic and rendering
├─ parser.rs      # Beatmap parsing and audio loading
├─ models.rs      # Game state, notes, and hit counts
```

### Key Data Structures

#### `GameState`

```rust
pub struct GameState {
    pub notes: Vec<Note>,
    pub score: i32,
    pub combo: i32,
    pub song_finished: bool,
    pub song_duration: f32,
    ...
}
```

#### `Note`

```rust
pub struct Note {
    pub start_time: f32,
    pub end_time: f32,
    pub lane: usize,
    pub hit: bool,
    pub missed: bool,
    pub ln_started: bool,
    pub ln_completed: bool,
}
```

#### `HitCounts`

```rust
pub struct HitCounts {
    pub perfect: i32,
    pub great: i32,
    pub good: i32,
    pub ok: i32,
    pub miss: i32,
}
```

---

## Usage Instructions

1. **Run the game**

```bash
cargo run
```

2. **Menu Options**

   * Select **2K or 4K mode**.
   * Import `.osz` files via the “Import” button.
   * Select a beatmap folder to see difficulties.

3. **Gameplay**

   * Hit the correct key when the note reaches the hit zone.
   * Hold keys for long notes.
   * Watch the judgment and combo at the top of the screen.

4. **Results**

   * After the song ends, results are displayed automatically.
   * Press ESC to return to difficulty select.

---

## Limitations / Weak Points

* **No key rebinding menu** – player must use default keys.
* **No hit sound effects** – only music plays.
* **Minimal Menu UI** – no scrolling for many beatmaps, no previews, no animations.
* **No pause or restart functionality**.
* **Limited to 4K** – no support for 6K or 7K.
* **No persistent storage for scores** – results reset on restart.
* **No visual effects for note hits beyond simple coloring.**
* Currently **no settings menu**.
* **UI and UX are very basic**.

---

## Future Plans / Improvements

1. **Key Rebinding Menu**

   * Allow players to change lane keys in-game.

2. **Hit Sound Effects**

   * Add feedback for hits and misses.

3. **Highscore & Replay System**

   * Save top scores and accuracy.
   * Optional replay playback.

4. **Enhanced Menu UI**

   * Scrollable beatmap list.
   * Beatmap previews.
   * Animations and particle effects.

5. **Additional Modes**

   * Extend beyond 4K.
   * Customizable scroll speed, judge windows, or modifier keys.

6. **Skins / Visual Customization**

   * Allow players to change note colors, backgrounds, and playfield.

7. **Map Editing / Mapping Tools**

   * Enable creating custom maps within the game.
   * Export/import for sharing.

8. **Settings Menu**

   * Add a proper menu to adjust key bindings, scroll speed, volume, and other gameplay options.

---

## Conclusion

Rustania is a functional rhythm game engine with 2K/4K support, long notes, scoring, combo system, and results screen. As a **school project**, it demonstrates parsing files, handling input, audio playback, and real-time gameplay logic.

Planned features like **skinning, mapping tools, settings, and more polished UI** will make it a more complete rhythm game experience in the future.
