use anyhow::{Context, Result, bail};

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

impl Default for Display {
    fn default() -> Self {
        Self::new()
    }
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
        if row >= DISPLAY_ROWS || col >= DISPLAY_COLS {
            bail!("Tried to set outside display bounds!")
        }
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        *el = val;
        Ok(())
    }

    /// Get the element of the display at the specified row and column
    pub fn get(&self, row: usize, col: usize) -> Result<bool> {
        if row >= DISPLAY_ROWS || col >= DISPLAY_COLS {
            bail!("Tried to get outside display bounds!")
        }
        return Ok(*(self
            .data
            .get(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?));
    }

    /// XOR the element at the specified row and column
    /// returns true if value was turned from set to unset
    pub fn xor(&mut self, row: usize, col: usize, val: bool) -> Result<bool> {
        if row >= DISPLAY_ROWS || col >= DISPLAY_COLS {
            bail!("Tried to xor outside display bounds!")
        }
        let el = self
            .data
            .get_mut(row * ROW_STRIDE + col * COL_STRIDE)
            .context("Tried to index past display bounds!")?;
        let flip = *el & val;
        *el ^= val;
        Ok(flip)
    }

    /// Return an iterator over the elements of the display
    pub fn iter_cells(&self) -> std::slice::Iter<'_, bool> {
        self.data.iter()
    }

    /// Clear the display (set every pixel to 0)
    pub fn clear(&mut self) -> Result<()> {
        self.data.fill(false);
        Ok(())
    }
}

#[cfg(test)]
mod test_display {
    use super::*;

    #[test]
    /// Test creating a display
    fn test_create() {
        let test_display = Display::new();

        for cell in test_display.data {
            assert!(!cell)
        }
    }

    #[test]
    /// Test setting a value on the display
    fn test_set() -> Result<()> {
        let mut test_display = Display::new();

        // Set the 0,0 to 1
        test_display.set(0, 0, true)?;
        assert!(test_display.data[0]);

        // Set the 1, 0 to 1
        test_display.set(1, 0, true)?;
        assert!(test_display.data[DISPLAY_COLS]);

        // Set the 0, 20 to 1
        test_display.set(0, 20, true)?;
        assert!(test_display.data[20]);

        // SEt the 10, 20 to 1
        test_display.set(10, 20, true)?;
        assert!(test_display.data[10 * DISPLAY_COLS + 20]);

        Ok(())
    }

    #[test]
    /// Test getting a value from the display
    fn test_get() -> Result<()> {
        let mut test_display = Display::new();

        // Set the 0,0 to 1
        test_display.set(0, 0, true)?;
        assert!(test_display.get(0, 0)?);

        // Set the 1, 0 to 1
        test_display.set(1, 0, true)?;
        assert!(test_display.get(1, 0)?);

        // Set the 0, 20 to 1
        test_display.set(0, 20, true)?;
        assert!(test_display.get(0, 20)?);

        // SEt the 10, 20 to 1
        test_display.set(10, 20, true)?;
        assert!(test_display.get(10, 20)?);

        Ok(())
    }

    #[test]
    /// Test xoring a value on the display
    fn test_xor() -> Result<()> {
        let mut test_display = Display::new();

        // xor the 10, 20 with 0, leaving it off
        assert!(!test_display.xor(10, 20, false)?);
        assert!(!test_display.get(10, 20)?);

        // xor the 10, 20 with 1, turning it on
        assert!(!test_display.xor(10, 20, true)?);
        assert!(test_display.get(10, 20)?);

        // xor the 10, 20 with 0, leaving it on
        assert!(!test_display.xor(10, 20, false)?);
        assert!(test_display.get(10, 20)?);

        // xor the 10, 20 with 1, turning it on
        assert!(test_display.xor(10, 20, true)?);
        assert!(!test_display.get(10, 20)?);

        Ok(())
    }

    #[test]
    /// Test clearing the screen
    fn test_clear() -> Result<()> {
        let mut test_display = Display::new();

        // Turn on some test cells
        test_display.set(0, 0, true)?;
        test_display.set(DISPLAY_ROWS - 1, 0, true)?;
        test_display.set(0, DISPLAY_COLS - 1, true)?;
        test_display.set(DISPLAY_ROWS - 1, DISPLAY_COLS - 1, true)?;

        // Clear the screen
        test_display.clear()?;

        for cell in test_display.data {
            assert!(!cell);
        }

        Ok(())
    }

    #[test]
    /// Test that the bound are as expected/error returned when accessing outside of them
    fn test_bounds() -> Result<()> {
        let mut test_display = Display::new();

        if test_display.set(DISPLAY_ROWS, 0, true).is_ok() {
            panic!();
        }
        if test_display.set(0, DISPLAY_COLS + 1, true).is_ok() {
            panic!();
        }

        Ok(())
    }
}
