//! Sends lines of stdin to the device using the "Ping" command, and prints
//! the device's response to stdout.

mod common;

use std::io::{stdin, BufRead};

fn main() -> anyhow::Result<()> {
    let mut device = common::initialize()?;

    let stdin = stdin();
    let handle = stdin.lock();
    for result in handle.lines() {
        let line = result?;
        let received = device.ping(line.as_bytes())?;
        println!("{}", String::from_utf8_lossy(&received));
    }

    Ok(())
}
