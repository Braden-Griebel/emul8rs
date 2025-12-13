use anyhow::{Context, Result};

// Display Constants
pub const DISPLAY_ROWS: usize = 32;
pub const DISPLAY_COLS: usize = 64;
const COL_STRIDE: usize = 1;
const ROW_STRIDE: usize = DISPLAY_COLS;

// NOTE: This may be replaces with underlying bitvec to save space eventually

/// A boolean array representing the state of the display
pub struct Display {
    /// Underlying data representing the display (row major matrix)
    data: [bool; DISPLAY_ROWS * DISPLAY_COLS],
    /// Whether the display needs to be redrawn
    pub needs_redraw: bool,
}

impl Display {
    /// Create an empty display
    pub fn new() -> Self {
        Display {
            data: [false; DISPLAY_ROWS * DISPLAY_COLS],
            needs_redraw: false,
        }
    }

    /// Set a value in the display
    pub fn set(&mut self, row: usize, col: usize, val: bool) -> Result<()> {
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        *el = val;
        Ok(())
    }

    /// Get the element of the display at the specified row and column
    pub fn get(&self, row: usize, col: usize) -> Result<bool> {
        return Ok(*(self
            .data
            .get(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?));
    }

    /// XOR the element at the specified row and column
    pub fn xor(&mut self, row: usize, col: usize, val: bool) -> Result<()> {
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        *el ^= val;
        Ok(())
    }

    /// Return an iterator over the elements of the display
    pub fn iter(&self) -> std::slice::Iter<'_, bool> {
        self.data.iter()
    }

    /// Clear the display (set every pixel to 0)
    pub fn clear(&mut self) -> Result<()> {
        self.data.fill(false);
        Ok(())
    }
}
