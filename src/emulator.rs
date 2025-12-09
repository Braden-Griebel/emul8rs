use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};

const NUM_ROWS: usize = 64;
const NUM_COLS: usize = 128;
const COL_STRIDE: usize = 1;
const ROW_STRIDE: usize = 128;

/// A boolean array representing the state of the display
pub(crate) struct Display {
    /// Underlying data representing the display (row major matrix)
    data: [bool; NUM_ROWS * NUM_COLS],
}

impl Display {
    /// Create an empty display
    fn new() -> Self {
        Display {
            data: [false; NUM_ROWS * NUM_COLS],
        }
    }

    fn set(&mut self, row: usize, col: usize, val: bool) -> Result<()> {
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        *el = val;
        Ok(())
    }
}

/// Chip8 Emulator
pub(crate) struct Emulator {
    /// Memory including program memory and ram
    memory: Vec<u8>,
    /// Representation of the display (actual drawing handled in [crate::artist])
    display: Display,
    /// Pointer to current instruction (indexes memory)
    program_counter: usize,
    /// Index register (indexes memory)
    index_register: u16,
    /// Stack used to call subroutines/functions and return from them
    stack: [u16; 128],
    /// Current top of the stack (indexes stack)
    stack_top: usize,
    /// Timer decremented at 60Hz until it reaches 0
    delay_timer: Arc<Mutex<u8>>,
    /// Timer decremented at 60Hz until it reaches 0,
    /// gives off beeping sound while not 0
    sound_timer: Arc<Mutex<u8>>,
    /// General purpose registers (V0-VF)
    registers: [u8; 16],
    /// Handle of thread used for ticking the delay timers
    ticker_handle: Option<thread::JoinHandle<()>>,
}
