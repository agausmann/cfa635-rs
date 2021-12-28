//! Clears all text from the screen.

mod common;

fn main() -> anyhow::Result<()> {
    let mut device = common::initialize()?;
    device.clear_screen()?;
    Ok(())
}
