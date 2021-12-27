pub mod codec;

use self::codec::{Packet, PacketCodec, ReadPacketError, WritePacketError, MAX_DATA_LEN};
use serialport::SerialPort;
use std::time::Duration;
use thiserror::Error;

pub const ROWS: u8 = 4;
pub const COLUMNS: u8 = 20;

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
        let response = self.recv()?;
        if response.packet_type() == 0x40 | packet.packet_type() {
            Ok(response)
        } else {
            Err(Error::BadResponse)
        }
    }

    pub fn ping(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        // Max data is 16 bytes.
        let payload = &data[..data.len().min(16)];
        let pong = self.transact(&Packet::new(0x00, payload))?;
        Ok(pong.data().to_owned())
    }

    pub fn clear_screen(&mut self) -> Result<(), Error> {
        self.transact(&Packet::new(0x06, &[]))?;
        Ok(())
    }

    pub fn set_cursor_position(&mut self, row: u8, col: u8) -> Result<(), Error> {
        if row >= ROWS || col >= COLUMNS {
            return Err(Error::InvalidArgument);
        }
        self.transact(&Packet::new(0x0b, &[col, row]))?;
        Ok(())
    }

    pub fn set_cursor_style(&mut self, style: CursorStyle) -> Result<(), Error> {
        self.transact(&Packet::new(0x0c, &[style as u8]))?;
        Ok(())
    }

    pub fn set_text(&mut self, row: u8, col: u8, text: &[u8]) -> Result<(), Error> {
        if row >= ROWS || col >= COLUMNS {
            return Err(Error::InvalidArgument);
        }
        // 20 bytes at most.
        let text = &text[..text.len().min(MAX_DATA_LEN - 2)];

        let mut buffer = [0; MAX_DATA_LEN];
        let len = 2 + text.len();
        buffer[0] = col;
        buffer[1] = row;
        buffer[2..len].copy_from_slice(&text);
        self.transact(&Packet::new(0x1f, &buffer[..len]))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CursorStyle {
    NoCursor = 0,
    BlinkingBlock = 1,
    StaticUnderscore = 2,
    BlinkingUnderscore = 3,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("serialport: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("potential desync on serial port")]
    Desync,

    #[error("received incorrect response for command")]
    BadResponse,

    #[error("invalid value for argument")]
    InvalidArgument,
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
