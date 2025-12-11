// Std uses
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

// External uses
use anyhow::{Context, Result, bail};
use raylib::{RaylibHandle, ffi::KeyboardKey};

// Display Constants
const DISPLAY_ROWS: usize = 32;
const DISPLAY_COLS: usize = 64;
const COL_STRIDE: usize = 1;
const ROW_STRIDE: usize = DISPLAY_COLS;

// Emulator constants
const MAX_STACK_SIZE: usize = 128;
const NUM_REGISTERS: usize = 16;
const MILLIS_PER_SECOND: u64 = 1_000;
const TIMER_HZ: u64 = 60;
const GAME_MEMORY_START: usize = 0x200;
const INSTRUCTION_LENGTH: usize = 2;

// Sprite constants
const SPRITE_WIDTH: usize = 8;

// Font
const FONT_START_POSITION: usize = 0x50;
const FONT_HEIGHT: usize = 5;
const FONT_CHAR_COUNT: usize = 16;
const FONT: [u8; FONT_HEIGHT * FONT_CHAR_COUNT] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

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

// NOTE: This may be replaces with underlying bitvec to save space eventually

/// A boolean array representing the state of the display
pub(crate) struct Display {
    /// Underlying data representing the display (row major matrix)
    data: [bool; DISPLAY_ROWS * DISPLAY_COLS],
    /// Whether the display needs to be redrawn
    needs_redraw: bool,
}

impl Display {
    /// Create an empty display
    fn new() -> Self {
        Display {
            data: [false; DISPLAY_ROWS * DISPLAY_COLS],
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

    /// XOR the element at the specified row and column
    fn xor(&mut self, row: usize, col: usize, val: bool) -> Result<()> {
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        *el ^= val;
        Ok(())
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
    ticker_channel: Option<mpsc::Sender<()>>,
    /// Handle for performing Raylib operations
    raylib: RaylibHandle,
}

impl Drop for Emulator {
    /// Drop the emulator (just stops the counter thread)
    fn drop(&mut self) {
        // Send the stop to the ticker
        if let Some(channel) = &self.ticker_channel {
            channel.send(()).expect("Failed to stop ticker thread");
        }
        // Join the ticker back to this thread
        if let Some(handle) = self.ticker_handle.take() {
            handle.join().expect("Failed to join with ticker thread");
        }
    }
}

impl Emulator {
    /// Create a new Emulator with zeroed fields
    fn new(raylib: RaylibHandle) -> Result<Self> {
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

        // Create the empty memory, add the font characters
        let memory: Vec<u8> = Vec::with_capacity(4096);

        // Create the empty display
        let display = Display::new();

        let mut emulator = Self {
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
            ticker_channel: Some(sender),
            raylib,
        };
        emulator.load_font().context("Trying to create emulator")?;
        Ok(emulator)
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

    /// Load the font into memory starting at FONT_START_POSITION
    fn load_font(&mut self) -> Result<()> {
        for (idx, byte) in (FONT_START_POSITION..(FONT_START_POSITION + FONT.len())).zip(FONT) {
            *(self
                .memory
                .get_mut(idx)
                .context("Trying to load font into emulator memory")?) = byte
        }
        Ok(())
    }

    /// Read a file, loads into memory starting at position 0x200 (512)
    fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let contents = std::fs::read(path).context("Failed to read input file")?;
        let mut memory_index: usize = 0x200;

        // Iterate through the file, moving each byte into memory
        for byte in contents {
            *(self
                .memory
                .get_mut(memory_index)
                .context("Insufficient memory to hold game file")?) = byte;
            memory_index += 1;
        }

        Ok(())
    }

    /// Draw a sprite to the screen
    ///
    /// Starting from the byte in memory at sprite_index, with length/height sprite_length,
    /// draw the sprite at the row given by y_pos, and the columns given by x_pos.
    fn draw_sprite(
        &mut self,
        sprite_index: usize,
        sprite_length: usize,
        x_pos: usize,
        y_pos: usize,
    ) -> Result<()> {
        let mut cur_index = sprite_index;
        // The x and y coordinates are allowed to wrap
        let x_pos = x_pos % DISPLAY_COLS;
        let y_pos = y_pos % DISPLAY_ROWS;

        // Loop through the sprite, xoring with the display bits
        for row_offset in 0..sprite_length {
            // If off bottom of screen, stop trying to draw
            if y_pos + row_offset >= DISPLAY_ROWS {
                break;
            };
            // Get the byte for the current row of the sprite
            let mut sprite_byte = self
                .memory
                .get(cur_index)
                .context("Trying to get byte in sprite")?
                .to_owned();
            for col_offset in 0..SPRITE_WIDTH {
                // Stop trying to draw if going off screen
                if x_pos + col_offset >= DISPLAY_COLS {
                    break;
                };
                // XOR the display bit with the value of the sprite at this index
                // offset (tracked by shifting the sprite byte to the left)
                self.display.xor(
                    y_pos + row_offset,
                    x_pos + col_offset,
                    (sprite_byte & 0b10000000) == 0b10000000,
                )?;
                // Shift the sprite_byte, which will result in the bit of interest being
                // at the most significant position
                sprite_byte <<= 1;
            }
            // Increment the memory index
            cur_index += 1;
        }
        Ok(())
    }

    /// Check if the `key` is currently pressed
    ///
    /// Key is a u8, representing one of the 0-F keys
    /// mapped from
    /// 1  2  3  4
    /// Q  W  E  R
    /// A  S  D  F
    /// Z  X  C  V
    /// to
    /// 1  2  3  C
    /// 4  5  6  D
    /// 7  8  9  E
    /// A  0  B  F
    fn check_key(&self, key: u8) -> Result<bool> {
        // If bounds check gaurunteed by the u8 passed in
        Ok(self.raylib.is_key_down(KEYMAP[key as usize]))
    }

    /// Jump to provided destination
    fn jump(&mut self, dest: usize) -> Result<()> {
        self.program_counter = dest;
        Ok(())
    }

    /// Get the value in register `register`
    fn get_reg(&self, register: usize) -> Result<u8> {
        Ok(self
            .registers
            .get(register)
            .context(format!("Trying to get value at register {register:#x}"))?
            .to_owned())
    }

    /// Set the value in register `register` to `value`
    fn set_reg(&mut self, register: usize, value: u8) -> Result<()> {
        // Bounds check to indicate panic
        if register >= NUM_REGISTERS {
            bail!("Trying to get value at register {register:#x}")
        }
        self.registers[register] = value;
        Ok(())
    }

    /// Add the value in register `register` to `value`
    fn add_reg(&mut self, register: usize, value: u8) -> Result<()> {
        // Bounds check to indicate panic
        if register >= NUM_REGISTERS {
            bail!("Trying to get value at register {register:#x}")
        };
        self.registers[register] += value;
        Ok(())
    }

    /// Fetch the current instruction (incrementing the program counter appropriately)
    fn fetch(&mut self) -> Result<(u8, u8)> {
        let b1 = self
            .memory
            .get(self.program_counter)
            .context("Trying to fetch first byte of instruction")?
            .to_owned();
        let b2 = self
            .memory
            .get(self.program_counter + 1)
            .context("Trying to fetch second byte of instruction")?
            .to_owned();
        self.program_counter += INSTRUCTION_LENGTH;
        Ok((b1, b2))
    }

    /// Execute a single instruction
    fn execute(&mut self) -> Result<()> {
        // Gets the instruction, increments the program counter
        let (instruction_byte1, instruction_byte2) = self.fetch()?;

        // Decode the instruction into various nibbles (half bytes), other values
        let nib1 = (instruction_byte1 & 0xF0) >> 4; // Used to determine instruction type
        let nib_x = instruction_byte1 & 0x0F; // Used for register address
        let nib_y = (instruction_byte2 & 0xF0) >> 4; // Used for register address
        let nib_n = instruction_byte2 & 0x0F; // 4 bit number
        // Other bit combinations used, not really nibbles but convienient prefix
        let nib_nn = (nib_x << 4) | nib_y; // 8-bit immediate number (not index)
        let nib_nnn: u16 = ((nib_x as u16) << 8) | ((nib_y as u16) << 4) | (nib_n as u16);
        // Match on the instruction (breaking it down by half-bytes as that
        // is how instructions are distinguished)
        let _: () = match (nib1, nib_x, nib_y, nib_n) {
            // CLEAR
            (0x0, 0x0, 0xE, 0x0) => {
                self.display.clear()?;
                self.display.needs_redraw = true;
            }
            // JUMP
            (0x1, ..) => {
                self.jump(nib_nnn as usize)?;
            }
            // SUBROUTINE
            (0x2, ..) => {
                // Push pc onto stack for returning from subrouting
                self.stack_push(self.program_counter as u16)?;
                // Jump to destination
                self.jump(nib_nnn as usize)?;
            }
            // RETURN
            (0x0, 0x0, 0xE, 0xE) => {
                let dest = self.stack_pop()? as usize;
                self.jump(dest)?;
            }
            // Conditional jumps
            (0x3, x, ..) => {
                // If value of register VX is equal to NN, skip next instruction
                if self.get_reg(x as usize)? == nib_nn {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            (0x4, x, ..) => {
                // If value of register VX is NOT equal to NN, skip next instruction
                if self.get_reg(x as usize)? != nib_nn {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            (0x5, x, y, ..) => {
                // If value at VX == value at VY, skip next instruction
                if self.get_reg(x as usize)? == self.get_reg(y as usize)? {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            (0x9, x, y, ..) => {
                // If value at VX != value at VY, skip next instruction
                if self.get_reg(x as usize)? != self.get_reg(y as usize)? {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            // Set Register
            (0x6, x, ..) => {
                self.set_reg(x as usize, nib_nn)?;
            }
            // Add to register
            (0x7, x, ..) => {
                self.add_reg(x as usize, nib_nn)?;
            }
            (other, ..) => {
                bail!("Instruction {other:#x} not implemented")
            }
        };
        Ok(())
    }
}
