pub mod codec;

use self::codec::PacketCodec;
use serialport::SerialPort;

pub struct Device {
    codec: PacketCodec<Box<dyn SerialPort>>,
}

impl Device {
    pub fn new(serial_port: Box<dyn SerialPort>) -> Self {
        Self {
            codec: PacketCodec::new(serial_port),
        }
    }
}
