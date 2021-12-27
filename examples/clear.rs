use anyhow::Context;
use cfa635::Device;
use std::env::args;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut args = args();
    let path = args.nth(1).context(USAGE)?;
    let mut device = Device::new(path)?;
    device.clear_screen()?;

    Ok(())
}

const USAGE: &str = "usage: [port]";
