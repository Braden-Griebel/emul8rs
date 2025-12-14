use crate::frontend::Frontend;

/// An empty frontend to use when testing the emulator
pub struct NoOpFrontend {}

impl NoOpFrontend {
    pub fn new() -> Self {
        Self {}
    }
}

impl Frontend for NoOpFrontend {
    fn draw(&mut self, _display: &crate::display::Display) -> anyhow::Result<()> {
        Ok(())
    }

    fn check_key(&mut self, _key: u8) -> anyhow::Result<bool> {
        Ok(false)
    }

    fn play_sound(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn stop_sound(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn should_stop(&mut self) -> bool {
        true
    }

    fn step(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
