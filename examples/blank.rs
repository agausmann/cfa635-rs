//! Blanks the display by turning off the backlights on the given device.

mod common;

fn main() -> anyhow::Result<()> {
    let mut device = common::initialize()?;
    device.set_backlight(0, 0)?;
    Ok(())
}
