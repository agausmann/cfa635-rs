//! Initialization and device acquisition that is common to most examples.

use anyhow::Context;
use cfa635::Device;
use std::env;

pub fn initialize() -> anyhow::Result<Device> {
    env_logger::init();

    let device_path = env::args()
        .nth(1)
        .or_else(|| env::var("CFA_DEVICE").ok())
        .context(NO_DEVICE_PATH)?;

    let device = Device::new(device_path)?;
    Ok(device)
}

const NO_DEVICE_PATH: &str = "No device path specified.\nEither provide it as \
    the first argument, or set the CFA_DEVICE environment variable.";
