use anyhow::Context;
use cfa635::Device;
use std::env::args;
use std::io::{stdin, BufRead};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut args = args();
    let path = args.nth(1).context(USAGE)?;
    let mut device = Device::new(path)?;

    let stdin = stdin();
    let handle = stdin.lock();
    for result in handle.lines() {
        let line = result?;
        device.clear_screen()?;
        device.set_text(0, 0, line.as_bytes())?;
    }

    Ok(())
}

const USAGE: &str = "usage: [port]";
