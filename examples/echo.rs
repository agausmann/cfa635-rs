//! Sends lines of stdin to the device using the "Ping" command, and prints
//! the device's response to stdout.

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
        let received = device.ping(line.as_bytes())?;
        println!("{}", String::from_utf8_lossy(&received));
    }

    Ok(())
}

const USAGE: &str = "usage: [port]";
