#![allow(dead_code)]
pub mod display;
pub mod emulator;
pub mod frontend;
#[cfg(feature = "raylib")]
mod raylib_frontend;

fn main() {
    println!("Hello, world!");
}
