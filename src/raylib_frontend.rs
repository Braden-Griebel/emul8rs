use raylib::{
    RaylibHandle,
    audio::{RaylibAudio, Sound},
    color::Color,
    drawing::RaylibDrawHandle,
    ffi::KeyboardKey,
    prelude::RaylibDraw,
};

use anyhow::Result;

use crate::display::{DISPLAY_COLS, DISPLAY_ROWS, Display};
use crate::frontend::Frontend;
// Keymap
// mapped from
// 1  2  3  4
// Q  W  E  R
// A  S  D  F
// Z  X  C  V
// to
// 1  2  3  C
// 4  5  6  D
// 7  8  9  E
// A  0  B  F
const KEYMAP: [KeyboardKey; 16] = [
    KeyboardKey::KEY_X,
    KeyboardKey::KEY_ONE,
    KeyboardKey::KEY_TWO,
    KeyboardKey::KEY_THREE,
    KeyboardKey::KEY_Q,
    KeyboardKey::KEY_W,
    KeyboardKey::KEY_E,
    KeyboardKey::KEY_A,
    KeyboardKey::KEY_S,
    KeyboardKey::KEY_D,
    KeyboardKey::KEY_Z,
    KeyboardKey::KEY_C,
    KeyboardKey::KEY_FOUR,
    KeyboardKey::KEY_R,
    KeyboardKey::KEY_F,
    KeyboardKey::KEY_V,
];

// Sound file to include
const BEEP_SOUND: &[u8; 63128] = include_bytes!("../resources/beep.wav");

// Window size defaults
const WINDOW_WIDTH: i32 = 640;
const WINDOW_HEIGHT: i32 = 480;

/// Fontend using the Raylib library
struct RaylibFrontend<'a, 'b> {
    handle: RaylibHandle,
    drawhandle: RaylibDrawHandle<'b>,
    sound: Sound<'a>,
    playing_sound: bool,
    window_width: i32,
    window_height: i32,
    foreground: Color,
    background: Color,
}

impl<'a, 'b> RaylibFrontend<'a, 'b> {
    /// Create a new raylib frontend struct from a raylib handle
    fn new(
        handle: RaylibHandle,
        drawhandle: RaylibDrawHandle<'b>,
        audio: &'a RaylibAudio,
        foreground: Color,
        background: Color,
    ) -> Result<Self> {
        let wave = audio.new_wave_from_memory(".wav", BEEP_SOUND)?;
        let sound = audio.new_sound_from_wave(&wave)?;
        Ok(Self {
            handle,
            drawhandle,
            sound,
            playing_sound: true,
            window_width: WINDOW_WIDTH,
            window_height: WINDOW_HEIGHT,
            foreground,
            background,
        })
    }
}

impl Frontend for RaylibFrontend<'_, '_> {
    fn draw(&mut self, display: &Display) -> anyhow::Result<()> {
        // Check window sizing
        if self.handle.is_window_resized() {
            self.window_width = self.handle.get_render_width();
            self.window_height = self.handle.get_render_height();
        }
        // Get the sizes of the individual cells
        let cell_width = self.window_width / (DISPLAY_COLS as i32);
        let cell_height = self.window_height / (DISPLAY_ROWS as i32);
        // Clear to screen and start adding the filled cells
        self.drawhandle.clear_background(self.background);
        // Iterate through each cell, and draw it to the screen
        // NOTE: The display is in row major order
        let mut row: usize;
        let mut col: usize;

        for (index, cell) in display.iter_cells().enumerate() {
            // Only draw anything if the cell is true
            if *cell {
                // Find which cell is being drawn
                row = index / DISPLAY_COLS;
                col = index % DISPLAY_COLS;
                // Find the x and y coordinates of the top left corner
                let x_coord = col as i32 * cell_width;
                let y_coord = row as i32 * cell_height;

                // Find the
                self.drawhandle.draw_rectangle(
                    x_coord,
                    y_coord,
                    cell_width,
                    cell_height,
                    self.foreground,
                );
            }
        }

        Ok(())
    }

    fn check_key(&mut self, key: u8) -> anyhow::Result<bool> {
        Ok(self.handle.is_key_down(KEYMAP[key as usize]))
    }

    fn play_sound(&mut self) -> anyhow::Result<()> {
        self.sound.play();
        self.playing_sound = true;
        Ok(())
    }

    fn stop_sound(&mut self) -> anyhow::Result<()> {
        if self.sound.is_playing() {
            self.sound.stop();
        }
        self.playing_sound = false;
        Ok(())
    }

    fn should_stop(&mut self) -> bool {
        self.handle.window_should_close()
    }

    fn step(&mut self) -> anyhow::Result<()> {
        // If we should be playing sound, make sure we are
        // raylib doesn't(?) allow for just looping the sound
        // so this checks every loop to ensure the sound is playing
        if self.playing_sound && !self.sound.is_playing() {
            self.sound.play();
        }

        Ok(())
    }
}
