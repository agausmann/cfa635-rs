//! Reads stdin line-by-line, and displays those lines on the screen.

mod common;
use std::io::{stdin, BufRead};

fn main() -> anyhow::Result<()> {
    let mut device = common::initialize()?;

    let stdin = stdin();
    let handle = stdin.lock();
    for result in handle.lines() {
        let line = result?;
        device.clear_screen()?;
        device.set_text(0, 15, line.as_bytes())?;
    }

    Ok(())
}
