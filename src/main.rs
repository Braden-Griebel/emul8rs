pub mod config;
pub mod display;
pub mod emulator;
pub mod frontend;
// Front end implementations
#[cfg(feature = "raylib")]
mod raylib_frontend;
use std::path::PathBuf;

#[cfg(feature = "raylib")]
use raylib::core::audio;

// External crate uses
use anyhow::Result;
use clap::Parser;
use colog::basic_builder;
use log::{LevelFilter, debug, info};

// Internal crate uses
use crate::config::EmulatorConfig;

// CLI struct
#[derive(Parser)]
#[command(version, about, long_about = None)]
/// A simple chip8 emulator with multiple possible frontends
///
/// Command line arguments override values from the config. In Chip8 each instruction
/// is two bytes, with 4 half-byte nibbles that are meaningful,
/// i.e.  each instruction is of the form SXYN, with S determining the instruction,
/// X/Y being registers to get values from, and N being an immediate u8 number. VX
/// and VY are used to refer to the values in the X and Y registers respectively.
struct Cli {
    /// Path to chip8 program to load
    program: PathBuf,

    /// Sets a custom configuration file
    #[arg(short, long, value_name = "CONFIG")]
    config: Option<PathBuf>,

    /// Turn on logging
    #[arg(short, long, action = clap::ArgAction::Count)]
    logging: u8,

    /// Foreground color (as unprefixed hexstring, e.g. FFFFFF)
    #[arg(short, long)]
    foreground: Option<String>,

    /// background color (as unprefixed hexstring, e.g. FFFFFF)
    #[arg(short, long)]
    background: Option<String>,

    /// Number of chip8 instructions to try and execute per second
    #[arg(long)]
    instructions_per_second: Option<u64>,

    /// Whether to shift value in Y register and move result into
    /// X register, or shift X inplace
    #[arg(long)]
    shift_use_vy: Option<bool>,

    /// Whether to use value from 0 register when performing jump with
    /// offset, or to use value from the X register instead.
    #[arg(long)]
    jump_offset_use_v0: Option<bool>,

    /// Whether to update the Index register when storing/loading
    /// registers into memory
    #[arg(long)]
    store_memory_update_index: Option<bool>,
}

fn main() -> Result<()> {
    // Get command line arguments
    let args = Cli::parse();

    // Setup logging
    let level_filter = match args.logging {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        3 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    basic_builder()
        .default_format()
        .filter_level(level_filter)
        .init();

    // Get configuration
    info!("Getting configuration from file");
    let mut emulator_config: EmulatorConfig;
    match args.config {
        Some(path) => {
            emulator_config = confy::load_path(path)?;
        }
        None => {
            emulator_config = confy::load("emul8rs", None)?;
        }
    };
    info!(
        "Default config file path: {:?}",
        confy::get_configuration_file_path("emul8rs", None)?
    );

    // Update config values if needed
    debug!("Updating config values with command line arguments");
    if let Some(foreground) = args.foreground.as_deref() {
        emulator_config.foreground = foreground.to_string();
    }
    if let Some(background) = args.background.as_deref() {
        emulator_config.background = background.to_string();
    }
    if let Some(ips) = args.instructions_per_second {
        emulator_config.instructions_per_second = ips;
    }
    if let Some(use_vy) = args.shift_use_vy {
        emulator_config.shift_use_vy = use_vy;
    }
    if let Some(use_v0) = args.jump_offset_use_v0 {
        emulator_config.jump_offset_use_v0 = use_v0;
    }
    if let Some(update_index) = args.store_memory_update_index {
        emulator_config.store_memory_update_index = update_index;
    }

    info!("Setting up frontend");
    cfg_if::cfg_if! {
        if #[cfg(feature = "raylib")]{
            info!("Setting up raylib");
            // Create the audio device the front end will use
            info!("Intializing the audio device");
            let raylib_audio = audio::RaylibAudio::init_audio_device()?;
            // Create the actual raylib frontend
            debug!("Initializing the raylib frontend");
            let frontend = raylib_frontend::RaylibFrontend::new(&emulator_config, &raylib_audio)?;
            // Create the emulator using the raylib front end
            info!("Initializing emulator");
            let mut emulator = emulator::Emulator::new(Box::new(frontend), emulator_config)?;
            info!("Loading game file");
            emulator.load_file(args.program)?;
            // Actually run the emulator using the raylib front end
            info!("Running the emulator");
            emulator.run()?;

        } else {
            warn!("No available fronends, exiting");
            println!("No Available Frontends!")
        }
    }
    Ok(())
}
