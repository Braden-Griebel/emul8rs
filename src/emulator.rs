// Std uses
use std::path::Path;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

// External uses
use anyhow::{Context, Result, bail};
use log::{debug, warn};
use rand::{self, RngCore};

// Crate uses
use crate::config;
use crate::display::{DISPLAY_COLS, DISPLAY_ROWS, Display};
use crate::frontend::Frontend;

// Emulator constants
const MAX_STACK_SIZE: usize = 128;
const MEMORY_SIZE: usize = 4096;
const NUM_REGISTERS: usize = 16;
const MILLIS_PER_SECOND: u64 = 1_000;
const MICROS_PER_SECOND: u64 = 1_000_000;
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

//NOTE: For the memory, the programs will be loaded starting at address 512

/// Chip8 Emulator
pub(crate) struct Emulator<'a> {
    /// Memory including program memory and ram
    memory: [u8; MEMORY_SIZE],
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
    frontend: Box<dyn Frontend + 'a>,
    /// Configuration object
    config: config::EmulatorConfig,
    /// Random number generator
    rng: rand::prelude::ThreadRng,
    /// Whether the emulator is currently playing sound
    playing_sound: bool,
    /// The length of time each instruction loop should take
    step_duration: Duration,
}

impl<'a> Drop for Emulator<'a> {
    /// Drop the emulator (just stops the counter thread)
    fn drop(&mut self) {
        // Send the stop to the ticker
        debug!("Stopping timer thread");
        if let Some(channel) = &self.ticker_channel {
            channel.send(()).expect("Failed to stop ticker thread");
        }
        // Join the ticker back to this thread
        if let Some(handle) = self.ticker_handle.take() {
            handle.join().expect("Failed to join with ticker thread");
        }
    }
}

impl<'a> Emulator<'a> {
    /// Create a new Emulator with zeroed fields
    pub fn new(frontend: Box<dyn Frontend + 'a>, config: config::EmulatorConfig) -> Result<Self> {
        // Create the sound and delay timers
        debug!("Creating timers");
        let delay_timer = Arc::new(Mutex::new(0u8));
        let sound_timer = Arc::new(Mutex::new(0u8));

        // Create the ticker which will decrement the delay and sound timer
        // Create the channel for sending th stop command
        debug!("Creating channel for stopping the timer");
        let (sender, receiver) = mpsc::channel();

        // Clone the delay and sound timer references to move them into the other thread
        debug!("Starting timer thread");
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
                match receiver.try_recv() {
                    Ok(_) => return, // Stop signal received
                    Err(mpsc::TryRecvError::Empty) => {
                        // No message received, fire the ticker
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

        // Create the empty memory, initialized to 0
        debug!("Initializing memory");
        let memory = [0u8; 4096];

        // Create the empty display
        debug!("Creating emulator internal display");
        let display = Display::new();

        // Create the RNG to use for randomness
        debug!("Creating the RNG");
        let rng = rand::rng();

        // Determine how long the execution steps should take
        let step_duration = Duration::from_micros(MICROS_PER_SECOND / 700);
        debug!(
            "Determined step duration to be {:?} microseconds",
            step_duration
        );

        debug!("Creating emulator object");
        let mut emulator = Self {
            memory,
            display,
            program_counter: GAME_MEMORY_START,
            index_register: 0,
            stack: [0u16; MAX_STACK_SIZE],
            stack_top: 0,
            registers: [0u8; NUM_REGISTERS],
            delay_timer,
            sound_timer,
            ticker_handle: Some(ticker_handle),
            ticker_channel: Some(sender),
            frontend,
            config,
            playing_sound: false,
            rng,
            step_duration,
        };
        debug!("Loading font into emulator");
        emulator.load_font().context("Trying to load font")?;
        Ok(emulator)
    }

    /// Run the emulator
    pub fn run(&mut self) -> Result<()> {
        debug!("Starting main emulation loop");
        while !self.frontend.should_stop() {
            // get the time at the start of the loop
            let start_time = Instant::now();
            self.frontend.draw(&self.display)?;
            self.execute()?;
            let sound_timer: u8;
            {
                sound_timer = *self.sound_timer.lock().unwrap();
            }
            if sound_timer > 0 && !self.playing_sound {
                self.frontend.play_sound()?;
                self.playing_sound = true;
            } else if sound_timer == 0 && self.playing_sound {
                self.frontend.play_sound()?;
                self.playing_sound = false;
            }
            let stop_time = Instant::now();
            // Sleep long enough to match the instructions per second
            thread::sleep(self.step_duration.saturating_sub(stop_time - start_time));
        }
        // Print the display (DEBUG TODO: Remove )
        for (idx, pixel) in self.display.iter_cells().enumerate() {
            if idx % DISPLAY_COLS == 0 {
                println!()
            }
            if *pixel {
                print!("◼");
            } else {
                print!("□");
            }
        }

        Ok(())
    }

    /// Read a file, loads into memory starting at position 0x200 (512)
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let contents = std::fs::read(path).context("Failed to read input file")?;
        self.load_bytes(&contents, GAME_MEMORY_START)?;
        Ok(())
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
        // Other bit combinations used, not really nibbles but convenient prefix
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
                // Push pc onto stack for returning from subroutine
                self.stack_push(self.program_counter as u16)?;
                // Jump to destination
                self.jump(nib_nnn as usize)?;
            }
            // RETURN
            (0x0, 0x0, 0xE, 0xE) => {
                let dest = self.stack_pop()? as usize;
                self.jump(dest)?;
            }
            // CONDITIONAL JUMPS
            (0x3, x, ..) => {
                // If value of register VX is equal to NN, skip next instruction
                if self.get_reg(x)? == nib_nn {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            (0x4, x, ..) => {
                // If value of register VX is NOT equal to NN, skip next instruction
                if self.get_reg(x)? != nib_nn {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            (0x5, x, y, ..) => {
                // If value at VX == value at VY, skip next instruction
                if self.get_reg(x)? == self.get_reg(y)? {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            (0x9, x, y, ..) => {
                // If value at VX != value at VY, skip next instruction
                if self.get_reg(x)? != self.get_reg(y)? {
                    self.program_counter += INSTRUCTION_LENGTH;
                }
            }
            // SET REGISTER
            (0x6, x, ..) => {
                self.set_reg(x as usize, nib_nn)?;
            }
            // ADD TO REGISTER
            (0x7, x, ..) => {
                self.add_reg(x as usize, nib_nn)?;
            }
            // ARITHMETIC/LOGICAL OPERATIONS
            // SET
            (0x8, x, y, 0x0) => {
                let vy_val = self.get_reg(y)?;
                self.set_reg(x as usize, vy_val)?;
            }
            // BINARY REGISTER OPS
            (0x8, x, y, n) => {
                let vx = self.get_reg(x)?;
                let vy = self.get_reg(y)?;
                match n {
                    0x1 => self.set_reg(x as usize, vx | vy)?,
                    0x2 => self.set_reg(x as usize, vx & vy)?,
                    0x3 => self.set_reg(x as usize, vx ^ vy)?,
                    0x4 => {
                        let (res, carry) = vx.overflowing_add(vy);
                        self.set_reg(0xF, carry.into())?;
                        self.set_reg(x as usize, res)?
                    }
                    0x5 => {
                        let (res, carry) = vx.overflowing_sub(vy);
                        self.set_reg(0xF, (!carry).into())?;
                        self.set_reg(x as usize, res)?
                    }
                    0x7 => {
                        let (res, carry) = vy.overflowing_sub(vx);
                        self.set_reg(0xF, (!carry).into())?;
                        self.set_reg(x as usize, res)?
                    }
                    0x6 | 0xE => {
                        let shift_right = n == 0x6;
                        // NOTE: Setting VX to VY is different between COSMAC and CHIP-48
                        let shift_target = if self.config.shift_use_vy { vy } else { vx };
                        // Shift register to the right
                        let dropped_bit =
                            shift_target & if shift_right { 0b00000001 } else { 0b10000000 };
                        // Set VX to shifted value
                        self.set_reg(
                            x as usize,
                            if shift_right {
                                shift_target >> 1
                            } else {
                                shift_target << 1
                            },
                        )?;
                        // Set flag register to dropped bit
                        self.set_reg(0xFusize, dropped_bit)?;
                    }
                    _ => bail!("Unimplemented binary register operation {:#x}", n),
                }
            }
            // SET INDEX REGISTER
            (0xA, ..) => self.set_index(nib_nnn)?,
            // JUMP WITH OFFSET
            (0xB, x, ..) => {
                // COSMAC jumped to NNN+V0, later jumped to NN+VX
                let dest = if self.config.jump_offset_use_v0 {
                    nib_nnn + self.get_reg(0x0)? as u16
                } else {
                    nib_nnn + self.get_reg(x)? as u16
                };
                self.program_counter = dest as usize;
            }
            // RAND
            (0xC, x, ..) => {
                // Get a random u8
                let rand: u8 = (self.rng.next_u32() >> (32 - 8)).try_into()?;
                // AND with the value NN
                self.set_reg(x as usize, rand & nib_nn)?;
            }
            // DISPLAY
            (0xD, x, y, n) => self.draw_sprite(
                self.get_index()?.into(),
                n as usize,
                self.get_reg(x)?.into(),
                self.get_reg(y)?.into(),
            )?,
            // SKIP IF KEY
            (0xE, x, 0x9, 0xE) => {
                if self.check_key(self.get_reg(x)?)? {
                    self.program_counter += INSTRUCTION_LENGTH
                };
            }
            // SKIP IF NOT KEY
            (0xE, x, 0xA, 0x1) => {
                if !self.check_key(self.get_reg(x)?)? {
                    self.program_counter += INSTRUCTION_LENGTH
                };
            }
            // TIMERS
            // GET DELAY TIMER
            (0xF, x, 0x0, 0x7) => {
                let current_timer: u8;
                // Lock and release as fast as possible, just grab the value
                {
                    current_timer = self.delay_timer.lock().unwrap().to_owned();
                }
                self.set_reg(x.into(), current_timer)?;
            }
            // SET DELAY TIMER
            (0xF, x, 0x1, 0x5) => {
                let new_delay = self.get_reg(x)?;
                {
                    *self.delay_timer.lock().unwrap() = new_delay;
                }
            }
            // SET SOUND TIMER
            (0xF, x, 0x1, 0x8) => {
                let new_delay = self.get_reg(x)?;
                {
                    *self.sound_timer.lock().unwrap() = new_delay;
                }
            }
            // ADD TO INDEX
            (0xF, x, 0x1, 0xE) => {
                let index = self.get_index()?;
                let (res, carry) = index.overflowing_add(self.get_reg(x)?.into());
                self.set_index(res)?;
                self.set_reg(0xF, (carry || res > 0x0FFF).into())?;
            }
            // BLOCKING GET KEY
            (0xF, x, 0x0, 0xA) => {
                let mut key_pressed = None;
                // Check if any of the keys are pressed
                for key in 0x0..=0xF {
                    if self.frontend.check_key(key)? {
                        key_pressed = Some(key);
                        break;
                    }
                }
                match key_pressed {
                    Some(key) => {
                        // NOTE: Key is guaranteed to fit into u8 since the length of the
                        // array is only 16
                        self.set_reg(x.into(), key)?;
                    }
                    None => {
                        // Set the program counter back to the start of this instruction
                        // to 'block' the program and wait for a key
                        self.program_counter -= INSTRUCTION_LENGTH;
                    }
                }
            }
            // SET INDEX TO FONT CHAR
            (0xF, x, 0x2, 0x9) => {
                self.set_index((FONT_START_POSITION + (x as usize * FONT_HEIGHT)).try_into()?)?;
            }
            // BINARY DECIMAL CONVERSION
            (0xF, x, 0x3, 0x3) => {
                // Get reg value
                let vx = self.get_reg(x)?;
                let idx = self.get_index()?;
                // Extract decimal
                let ones = vx % 10;
                let tens = (vx % 100) / 10;
                let hundreds = vx / 100;
                // Set the values in memory
                *(self
                    .memory
                    .get_mut(idx as usize)
                    .context("Memory index error during decimal conversion instruction")?) = ones;
                *(self
                    .memory
                    .get_mut((idx + 1) as usize)
                    .context("Memory index error during decimal conversion instruction")?) = tens;
                *(self
                    .memory
                    .get_mut((idx + 2) as usize)
                    .context("Memory index error during decimal conversion instruction")?) =
                    hundreds;
            }
            // STORE REGISTERS
            (0xF, x, 0x5, 0x5) => {
                let idx = self.get_index()? as usize;
                let vx = self.get_reg(x)?;
                for reg in 0..=vx {
                    let dest = idx + reg as usize;
                    *(self.memory.get_mut(dest).context(format!(
                        "Trying to store register {:#x} into memory at invalid address {:#x}",
                        x, dest,
                    ))?) = self.get_reg(reg)?;
                }
                if self.config.store_memory_update_index {
                    self.set_index(idx as u16 + x as u16 + 1)?;
                }
            }
            // LOAD REGISTERS
            (0xF, x, 0x6, 0x5) => {
                let idx = self.get_index()? as usize;
                let vx = self.get_reg(x)? as usize;
                for reg in 0..=vx {
                    let source = idx + reg;
                    self.set_reg(
                        reg,
                        *(self.memory.get(source).context(format!(
                            "Trying to load memory at invalid address {:#x} into register {:#x}",
                            source, x,
                        ))?),
                    )?;
                }
                if self.config.store_memory_update_index {
                    self.set_index(idx as u16 + x as u16 + 1)?;
                }
            }
            (other, ..) => {
                warn!("Instruction {other:#x} not implemented");
            }
        };
        Ok(())
    }
    /// Add a value to the stack
    fn stack_push(&mut self, value: u16) -> Result<()> {
        *(self
            .stack
            .get_mut(self.stack_top)
            .context("Stack overflow!")?) = value;
        self.stack_top += 1;
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
        self.load_bytes(&FONT, FONT_START_POSITION)
            .context("Loading font into memory")
    }

    fn load_bytes(&mut self, bytes: &[u8], start_position: usize) -> Result<()> {
        let mut memory_index = start_position;
        // Iterate through the file, moving each byte into memory
        for &byte in bytes {
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
        // Track if any bits were turned OFF
        let mut turned_off = false;

        // Loop through the sprite, XORing with the display bits
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
                // Stop trying to draw if going off-screen
                if x_pos + col_offset >= DISPLAY_COLS {
                    break;
                };
                // XOR the display bit with the value of the sprite at this index
                // offset (tracked by shifting the sprite byte to the left)
                if self.display.xor(
                    y_pos + row_offset,
                    x_pos + col_offset,
                    (sprite_byte & 0b10000000) == 0b10000000,
                )? {
                    turned_off = true;
                }
                // Shift the sprite_byte, which will result in the bit of interest being
                // at the most significant position
                sprite_byte <<= 1;
            }
            // Increment the memory index
            cur_index += 1;
        }
        if turned_off {
            self.set_reg(0xF, 1)?;
        }
        Ok(())
    }

    /// Check if the `key` is currently pressed
    fn check_key(&mut self, key: u8) -> Result<bool> {
        // If bounds check guaranteed by the u8 passed in
        self.frontend.check_key(key)
    }

    /// Jump to provided destination
    fn jump(&mut self, dest: usize) -> Result<()> {
        self.program_counter = dest;
        Ok(())
    }

    /// Get the value in register `register`
    fn get_reg(&self, register: u8) -> Result<u8> {
        Ok(self
            .registers
            .get(register as usize)
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

    /// Set the value of the index register
    fn set_index(&mut self, value: u16) -> Result<()> {
        self.index_register = value;
        Ok(())
    }

    /// Get the value of the index register
    fn get_index(&self) -> Result<u16> {
        Ok(self.index_register)
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
}

#[cfg(test)]
mod test_emulator {
    use super::*;

    use crate::{config::EmulatorConfig, noop_frontend::NoOpFrontend};

    #[test]
    /// Test creating the emulator
    fn test_create() -> Result<()> {
        let test_frontend = NoOpFrontend::new();
        let test_config = EmulatorConfig::default();
        let _test_eml8r = Emulator::new(Box::new(test_frontend), test_config)?;

        Ok(())
    }

    #[test]
    /// Test clearing the display
    fn test_clear() -> Result<()> {
        let test_frontend = NoOpFrontend::new();
        let test_config = EmulatorConfig::default();
        let mut test_emul8r = Emulator::new(Box::new(test_frontend), test_config)?;

        // Artifically set some cells of the display
        test_emul8r.display.set(0, 0, true)?;
        test_emul8r.display.set(10, 20, true)?;
        test_emul8r.display.set(3, 5, true)?;

        // Set the first instruction to be clear
        #[allow(clippy::identity_op)]
        {
            test_emul8r.memory[test_emul8r.program_counter] = (0x0 << 4) | 0x0;
            test_emul8r.memory[test_emul8r.program_counter + 1] = (0xE << 4) | 0x0;
        }
        // Run the single instruction
        test_emul8r.execute()?;

        // Check that the display has been cleared
        for &cell in test_emul8r.display.iter_cells() {
            assert!(!cell);
        }

        Ok(())
    }

    #[test]
    /// Test the stack memory
    fn test_stack() -> Result<()> {
        let test_frontend = NoOpFrontend::new();
        let test_config = EmulatorConfig::default();
        let mut test_emul8r = Emulator::new(Box::new(test_frontend), test_config)?;

        // Check that the stack is empty
        assert!(test_emul8r.stack_top == 0);

        // Push some numbers onto the stack
        test_emul8r.stack_push(5)?;
        test_emul8r.stack_push(10)?;
        test_emul8r.stack_push(1)?;
        test_emul8r.stack_push(0)?;
        test_emul8r.stack_push(50)?;

        // Check that stack top has moved forward/up
        assert_eq!(test_emul8r.stack_top, 5);

        // Check popping is correct
        assert_eq!(test_emul8r.stack_pop()?, 50);
        assert_eq!(test_emul8r.stack_pop()?, 0);
        assert_eq!(test_emul8r.stack_pop()?, 1);
        assert_eq!(test_emul8r.stack_pop()?, 10);
        assert_eq!(test_emul8r.stack_pop()?, 5);

        // Make sure the stack pointer has gone back to 0
        assert_eq!(test_emul8r.stack_top, 0);

        Ok(())
    }

    #[test]
    /// Test jump instruction
    fn test_jump() -> Result<()> {
        let test_frontend = NoOpFrontend::new();
        let test_config = EmulatorConfig::default();
        let mut test_emul8r = Emulator::new(Box::new(test_frontend), test_config)?;
        let jump_dest = 1012u16;

        // Set the first instruction to be clear
        #[allow(clippy::identity_op)]
        {
            let instruction1 = (0x1 << 4) | jump_dest >> 8;
            let instruction2 = jump_dest & 0xFF;
            println!("Jump dest is {:b}", jump_dest);
            println!("Instruction1 is {:b}", instruction1);
            println!("Instruction2 is {:b}", instruction2);

            test_emul8r.memory[test_emul8r.program_counter] = instruction1 as u8;
            test_emul8r.memory[test_emul8r.program_counter + 1] = instruction2 as u8;
        }
        // Run the single instruction
        test_emul8r.execute()?;

        // Check that the program counter has been set to 1012
        assert_eq!(test_emul8r.program_counter, jump_dest as usize);

        Ok(())
    }
}
