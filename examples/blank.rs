//! Blanks the display by turning off the backlights on the given device.

use anyhow::Context;
use cfa635::Device;
use std::env::args;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut args = args();
    let path = args.nth(1).context(USAGE)?;
    let mut device = Device::new(path)?;
    device.set_backlight(0, 0)?;

    Ok(())
}

const USAGE: &str = "usage: [port]";
