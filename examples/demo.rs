//! A more complete demo of the features of the CFA635.

mod common;

use cfa635::{Device, Key, Report};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    let device = common::initialize()?;
    let mut menu = Menu::new(device)?;
    menu.run()?;
    Ok(())
}

struct Menu {
    device: Device,
    entries: Vec<MenuEntry>,
    current_index: usize,
}

impl Menu {
    fn new(mut device: Device) -> anyhow::Result<Self> {
        device.configure_key_reporting(
            &[Key::Up, Key::Down, Key::Left, Key::Right],
            &[Key::Up, Key::Down],
        )?;
        device.clear_screen()?;

        let entries = vec![
            MenuEntry {
                name: pad(b"Backlight".to_vec()),
                value: 100,
                max_value: 100,
                setter: |dev, val| dev.set_backlight(val, val),
            },
            MenuEntry {
                name: pad(b"Contrast".to_vec()),
                value: 120,
                max_value: 254,
                setter: |dev, val| dev.set_contrast(val),
            },
            MenuEntry {
                name: pad(b"LED 0 (Green)".to_vec()),
                value: 0,
                max_value: 1,
                setter: |dev, val| dev.set_led(0, 0, val * 100),
            },
            MenuEntry {
                name: pad(b"LED 1 (Red)".to_vec()),
                value: 0,
                max_value: 1,
                setter: |dev, val| dev.set_led(1, val * 100, 0),
            },
            MenuEntry {
                name: pad(b"LED 2 (Yellow)".to_vec()),
                value: 0,
                max_value: 1,
                setter: |dev, val| dev.set_led(2, val * 100, val * 100),
            },
            MenuEntry {
                name: pad(b"LED 3 (Orange)".to_vec()),
                value: 0,
                max_value: 1,
                setter: |dev, val| dev.set_led(3, val * 100, val * 50),
            },
        ];

        for entry in &entries {
            (entry.setter)(&mut device, entry.value)?;
        }

        Ok(Self {
            device,
            entries,
            current_index: 0,
        })
    }

    fn run(&mut self) -> anyhow::Result<()> {
        loop {
            if let Some(report) = self.device.poll_report()? {
                self.handle(report)?;
            } else {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    fn handle(&mut self, report: Report) -> anyhow::Result<()> {
        match report {
            Report::KeyActivity { key, pressed } => match (key, pressed) {
                (Key::Left, true) => {
                    self.prev_entry()?;
                }
                (Key::Right, true) => {
                    self.next_entry()?;
                }
                (Key::Up, true) => {
                    self.next_value()?;
                }
                (Key::Down, true) => {
                    self.prev_value()?;
                }
                _ => {}
            },
        }
        Ok(())
    }

    fn entry(&self) -> &MenuEntry {
        &self.entries[self.current_index]
    }

    fn entry_mut(&mut self) -> &mut MenuEntry {
        &mut self.entries[self.current_index]
    }

    fn next_entry(&mut self) -> anyhow::Result<()> {
        self.set_current_index((self.current_index + 1) % self.entries.len())?;
        Ok(())
    }

    fn prev_entry(&mut self) -> anyhow::Result<()> {
        self.set_current_index((self.current_index + self.entries.len() - 1) % self.entries.len())?;
        Ok(())
    }

    fn next_value(&mut self) -> anyhow::Result<()> {
        let entry = self.entry();
        let next_value = (entry.value + 1).min(entry.max_value);
        self.set_value(next_value)?;
        Ok(())
    }

    fn prev_value(&mut self) -> anyhow::Result<()> {
        let entry = self.entry();
        let prev_value = entry.value.saturating_sub(1);
        self.set_value(prev_value)?;
        Ok(())
    }

    fn set_current_index(&mut self, idx: usize) -> anyhow::Result<()> {
        self.current_index = idx;
        self.send_name()?;
        self.send_value()?;
        Ok(())
    }

    fn set_value(&mut self, value: u8) -> anyhow::Result<()> {
        self.entry_mut().value = value;
        let setter = self.entry().setter;
        setter(&mut self.device, value)?;
        self.send_value()?;
        Ok(())
    }

    fn send_name(&mut self) -> anyhow::Result<()> {
        self.device
            .set_text(0, 0, &self.entries[self.current_index].name)?;
        Ok(())
    }

    fn send_value(&mut self) -> anyhow::Result<()> {
        self.device
            .set_text(1, 0, &pad(format_value(self.entry().value)))?;
        Ok(())
    }
}

struct MenuEntry {
    name: Vec<u8>,
    value: u8,
    max_value: u8,
    setter: fn(&mut Device, u8) -> Result<(), cfa635::Error>,
}

fn format_value(x: u8) -> Vec<u8> {
    x.to_string().into_bytes()
}

fn pad(mut v: Vec<u8>) -> Vec<u8> {
    v.resize(20, b' ');
    v
}
