use anyhow::Result;

use crate::display::Display;

/// Trait for implementing a front-end to the compiler,
/// will essentially need a way to draw the display,
/// read keyboard input, play a sound, and check if
/// the emulator should stop (any of these except the
/// should quit can technically be noops so the
/// emulator can be run in a headless mode)
pub trait Frontend {
    /// Take in a display object, and draw it
    ///
    /// Will pass the current state of the display,
    /// so the screen will likely need to be cleared,
    /// or some internal state can be used to check
    /// only update needed cells.
    fn draw(&mut self, display: &Display) -> Result<()>;
    /// Check if a key is down, returing Ok(true) if
    /// if is down, and Ok(false) if it isn't.
    ///
    /// The key will be a byte valued betwee
    /// 0x0 and 0xF, how these are mapped to an actual
    /// input is up to the frontend to decide.
    fn check_key(&mut self, key: u8) -> Result<bool>;
    /// Play a tone until [stop_sound] is called
    ///
    /// The tone can be anything that the frontend wants it to be.
    fn play_sound(&mut self) -> Result<()>;
    /// Stop playing the sound started by [start_sound]
    fn stop_sound(&mut self) -> Result<()>;
    /// Check if the emulator should exit
    fn should_stop(&mut self) -> bool;
    /// Function called during every instruction loop
    ///
    /// Mainly a workaround to allow raylib front end to keep the audio playing
    fn step(&mut self) -> Result<()>;
}
