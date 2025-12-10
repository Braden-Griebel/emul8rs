use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};

// Display Constants
const NUM_ROWS: usize = 32;
const NUM_COLS: usize = 64;
const COL_STRIDE: usize = 1;
const ROW_STRIDE: usize = 128;

// Emulator constants
const MAX_STACK_SIZE: usize = 128;
const NUM_REGISTERS: usize = 16;
const MILLIS_PER_SECOND: u64 = 1_000;
const TIMER_HZ: u64 = 60;

/// A boolean array representing the state of the display
pub(crate) struct Display {
    /// Underlying data representing the display (row major matrix)
    data: [bool; NUM_ROWS * NUM_COLS],
    /// Whether the display needs to be redrawn
    needs_redraw: bool,
}

impl Display {
    /// Create an empty display
    fn new() -> Self {
        Display {
            data: [false; NUM_ROWS * NUM_COLS],
            needs_redraw: false,
        }
    }

    /// Set a value in the display
    fn set(&mut self, row: usize, col: usize, val: bool) -> Result<()> {
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        *el = val;
        Ok(())
    }

    /// Get the element of the display at the specified row and column
    fn get(&self, row: usize, col: usize) -> Result<bool> {
        return Ok(*(self
            .data
            .get(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?));
    }

    /// Return an iterator over the elements of the display
    fn iter(&self) -> std::slice::Iter<'_, bool> {
        self.data.iter()
    }

    /// Clear the display (set every pixel to 0)
    fn clear(&mut self) -> Result<()> {
        self.data.fill(false);
        Ok(())
    }
}

//NOTE: For the memory, the programs will be loaded starting at adress 512

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
    stack: [u16; MAX_STACK_SIZE],
    /// Current top of the stack (indexes stack)
    stack_top: usize,
    /// Timer decremented at 60Hz until it reaches 0
    delay_timer: Arc<Mutex<u8>>,
    /// Timer decremented at 60Hz until it reaches 0,
    /// gives off beeping sound while not 0
    sound_timer: Arc<Mutex<u8>>,
    /// General purpose registers (V0-VF)
    registers: [u8; NUM_REGISTERS],
    /// Handle of thread used for ticking the delay timers
    ticker_handle: Option<thread::JoinHandle<()>>,
    /// Channel to the ticker thread
    ticker_channel: mpsc::Sender<()>,
}

impl Emulator {
    /// Create a new Emulator with zeroed fields
    fn new() -> Self {
        // Create the sound and delay timers
        let delay_timer = Arc::new(Mutex::new(0u8));
        let sound_timer = Arc::new(Mutex::new(0u8));

        // Create the ticker which will decrement the delay and sound timer
        // Create the channel for sending th stop command
        let (sender, reciever) = mpsc::channel();

        // Clone the delay and sound timer references to move them into the other thread
        let tickers_delay_timer_ref = delay_timer.clone();
        let tickers_sound_timer_ref = sound_timer.clone();
        let ticker_handle = thread::spawn(move || {
            // Create an Instant reference which will track when the ticker needs to fire
            let mut ticker = Instant::now();
            // Also track the previous tick so that the thread can sleep till it needs to fire again
            let mut previous_tick = Instant::now();
            // Find the period (based on the desired hertz) for ticking
            let period = Duration::from_millis(MILLIS_PER_SECOND / TIMER_HZ);

            loop {
                // Check if the thread has received a message (all messages are stops)
                match reciever.try_recv() {
                    Ok(_) => return, // Stop signal received
                    Err(mpsc::TryRecvError::Empty) => {
                        // No message recieved, fire the ticker
                        if ticker.elapsed() >= period {
                            // Decrement the timers
                            {
                                let mut delay_timer = tickers_delay_timer_ref.lock().unwrap();
                                *delay_timer = (*delay_timer).saturating_sub(1);
                            }
                            {
                                let mut sound_timer = tickers_sound_timer_ref.lock().unwrap();
                                *sound_timer = (*sound_timer).saturating_sub(1);
                            }
                            // Track the previous time (for sleeping the thread)
                            previous_tick = ticker;
                            // Set the current to the current timer
                            ticker = Instant::now();
                        }
                    }
                    Err(_) => return, // Channel has been disconnected
                }
                // Sleep until the next time tick is needed
                thread::sleep((previous_tick + period) - ticker);
            }
        });

        // Create the empty memory and display
        let memory: Vec<u8> = Vec::with_capacity(4096);
        let display = Display::new();

        Self {
            memory,
            display,
            program_counter: 0,
            index_register: 0,
            stack: [0u16; MAX_STACK_SIZE],
            stack_top: 0,
            registers: [0u8; NUM_REGISTERS],
            delay_timer,
            sound_timer,
            ticker_handle: Some(ticker_handle),
            ticker_channel: sender,
        }
    }

    /// Add a value to the stack
    fn stack_push(&mut self, value: u16) -> Result<()> {
        *(self
            .stack
            .get_mut(self.stack_top)
            .context("Stack overflow!")?) = value;
        Ok(())
    }

    /// Pop the value off the top of the stack
    fn stack_pop(&mut self) -> Result<u16> {
        if self.stack_top == 0 {
            bail!("Trying to pop from empty stack");
        }
        self.stack_top -= 1;
        Ok(*(self
            .stack
            .get(self.stack_top)
            .context("Invalid stack pointer")?))
    }
}
