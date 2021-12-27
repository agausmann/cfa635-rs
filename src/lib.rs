pub mod codec;

use self::codec::{Packet, PacketCodec, ReadPacketError, WritePacketError};
use serialport::SerialPort;
use std::time::Duration;
use thiserror::Error;

pub struct Device {
    codec: PacketCodec<Box<dyn SerialPort>>,
}

impl Device {
    pub fn new<P: AsRef<str>>(path: P) -> Result<Self, Error> {
        //TODO baud rate API - not relevant for USB version
        let port = serialport::new(path.as_ref(), 115200)
            .timeout(Duration::from_millis(250))
            .open()?;
        Ok(Self {
            codec: PacketCodec::new(port),
        })
    }

    fn send(&mut self, packet: &Packet) -> Result<(), Error> {
        log::trace!("sending {:?}", packet);
        self.codec.write_packet(packet)?;
        Ok(())
    }

    fn recv(&mut self) -> Result<Packet, Error> {
        let packet = self.codec.read_packet()?;
        log::trace!("received {:?}", packet);
        if packet.check_crc() {
            Ok(packet)
        } else {
            //TODO ignore+warn?
            Err(Error::Desync)
        }
    }

    fn transact(&mut self, packet: &Packet) -> Result<Packet, Error> {
        self.send(packet)?;
        //TODO buffer report packets
        self.recv()
    }

    pub fn ping(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        // Max data is 16 bytes.
        let payload = &data[..data.len().min(16)];
        let pong = self.transact(&Packet::new(0x00, payload))?;
        if pong.packet_type() != 0x40 {
            return Err(Error::BadResponse);
        }
        Ok(pong.data().to_owned())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("serialport: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("potential desync on serial port")]
    Desync,

    #[error("received incorrect response for command")]
    BadResponse,
}

impl From<WritePacketError> for Error {
    fn from(err: WritePacketError) -> Self {
        match err {
            WritePacketError::Io(err) => Self::Io(err),
        }
    }
}

impl From<ReadPacketError> for Error {
    fn from(err: ReadPacketError) -> Self {
        match err {
            ReadPacketError::Io(err) => Self::Io(err),
            ReadPacketError::InvalidPacket => Self::Desync,
        }
    }
}
