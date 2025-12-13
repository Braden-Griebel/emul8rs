#![allow(dead_code)]
pub mod display;
pub mod emulator;
pub mod frontend;
#[cfg(feature = "raylib")]
mod raylib_frontend;

use raylib::prelude::*;

fn main() {
    let (mut rl, thread) = raylib::init().size(640, 480).title("Hello, World").build();

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);
        d.draw_text("Hello, world!", 12, 12, 20, Color::BLACK);
    }
}
