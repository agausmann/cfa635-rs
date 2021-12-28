pub(crate) mod codec;

use self::codec::{Packet, PacketCodec, ReadPacketError, WritePacketError, MAX_DATA_LEN};
use serialport::SerialPort;
use std::collections::VecDeque;
use std::time::Duration;
use thiserror::Error;

/// How many rows (lines) the display has.
///
/// Rows are numbered starting with zero (0) at the top and increasing as you
/// move down. Acceptable values are in the range `0..NUM_ROWS` (note the
/// exclusive upper bound).
pub const NUM_ROWS: u8 = 4;

/// How many columns (characters per row) the display has.
///
/// Columns are numbered starting with zero (0) at the left and increasing as
/// you move right. Acceptable values are in the range `0..NUM_COLUMNS` (note
/// the exclusive upper bound).
pub const NUM_COLUMNS: u8 = 20;

/// How many indicator LEDs the display has.
///
/// Numbered starting with zero (0) at the top and increasing as you move down.
/// Acceptable valuse are in the range `0..NUM_LEDS` (note the exclusive upper
/// bound).
pub const NUM_LEDS: u8 = 4;

pub struct Device {
    codec: PacketCodec<Box<dyn SerialPort>>,
    report_buffer: VecDeque<Report>,
}

impl Device {
    /// Connect to a device using the named serial port.
    ///
    /// On Windows, the name is typically a COM device name (e.g. `COM1`).
    ///
    /// On Linux, the name is typically the path to the device (e.g.
    /// `/dev/ttyACM0` or `/dev/serial/by-id/...`)
    pub fn new<P: AsRef<str>>(path: P) -> Result<Self, Error> {
        //TODO baud rate API - not relevant for USB version
        let port = serialport::new(path.as_ref(), 115200)
            .timeout(Duration::from_millis(250))
            .open()?;
        Ok(Self {
            codec: PacketCodec::new(port),
            report_buffer: VecDeque::new(),
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
            Err(Error::InvalidRead)
        }
    }

    fn transact(&mut self, packet: &Packet) -> Result<Packet, Error> {
        self.send(packet)?;
        loop {
            let response = self.recv()?;
            let resp_class = response.packet_type() >> 6;
            let resp_code = response.packet_type() & 0x3f;
            if resp_class == 0b10 {
                if let Some(report) = Report::from_raw(&response) {
                    self.report_buffer.push_back(report);
                }
            } else if resp_class == 0b01 && resp_code == packet.packet_type() {
                // normal response code
                return Ok(response);
            } else if resp_class == 0b11 && resp_code == packet.packet_type() {
                // error response code
                return Err(Error::ReturnedError);
            } else {
                log::warn!("unexpected packet received: {:?}", response);
            }
        }
    }

    /// Sends a "Ping" with an arbitrary payload.
    ///
    /// If a correct response is received, this call will return `Ok` with the
    /// bytes sent back from the device. The data returned should be equal to
    /// the payload that was sent.
    ///
    /// Note: The maximum payload size is 16 bytes. If the provided data is
    /// longer, only the first 16 bytes will be sent (and therefore, only up to
    /// 16 bytes will be received).
    pub fn ping(&mut self, data: &[u8]) -> Result<Vec<u8>, Error> {
        // Max data is 16 bytes.
        let payload = &data[..data.len().min(16)];
        let pong = self.transact(&Packet::new(0x00, payload))?;
        Ok(pong.data().to_owned())
    }

    /// Saves the current state of the device as its "boot" state, i.e., the
    /// state that will be restored when the device powers on.
    ///
    /// The following items are saved and restored:
    ///
    /// - Characters shown on the LCD ([`Device::clear_screen`], [`Device::set_text`]).
    ///
    /// - Cursor position ([`Device::set_cursor_position`]).
    ///
    /// - Cursor style ([`Device::set_cursor_style`]).
    ///
    /// - Screen contrast ([`Device::set_contrast`]).
    ///
    /// - Screen backlight ([`Device::set_backlight`]).
    ///
    /// - Report configuration ([`Device::configure_key_reporting`])
    pub fn save_boot_state(&mut self) -> Result<(), Error> {
        self.transact(&Packet::new(0x04, &[]))?;
        Ok(())
    }

    /// Fills the screen with empty / space characters, and moves the cursor to
    /// the top-left character (row 0, column 0).
    pub fn clear_screen(&mut self) -> Result<(), Error> {
        self.transact(&Packet::new(0x06, &[]))?;
        Ok(())
    }

    /// Set the text on a region on the LCD screen, starting at the given position.
    ///
    /// If the text would be written past the right edge, it will be
    /// hard-wrapped to the next line.
    ///
    /// If there is already text at the given region, it will be overwritten.
    /// Any text outside of the region will be unaffected.
    ///
    /// Note: The maximum size of `text` is 20 bytes. If more bytes are passed,
    /// only the first 20 are written.
    ///
    /// Note 2: The display does not support arbitrary UTF-8. It is compatible
    /// with a subset of ASCII, specifically:
    ///
    /// - Alphanumerics `A-Z` `a-z'` and `0-9`
    ///
    /// - Space `' '`
    ///
    /// - Symbols `!"#%&'()*+,-./:;<=>?`
    ///
    /// For a complete table of supported characters, see [Section 8][cgrom] of
    /// the CFA635 datasheet.
    ///
    /// [cgrom]: https://www.crystalfontz.com/products/document/4131/CFA635-xxx-KU.pdf#%5B%7B%22num%22%3A140%2C%22gen%22%3A0%7D%2C%7B%22name%22%3A%22XYZ%22%7D%2C67%2C721%2C0%5D
    ///
    /// # Errors
    ///
    /// - `InvalidArgument` - If the row or column index is out of bounds (as
    /// defined by [`NUM_ROWS`] and [`NUM_COLUMNS`]).
    pub fn set_text(&mut self, row: u8, col: u8, text: &[u8]) -> Result<(), Error> {
        if row >= NUM_ROWS || col >= NUM_COLUMNS {
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

    /// Sets the cursor position to the character at the given row and column.
    ///
    /// # Errors
    ///
    /// - `InvalidArgument` - If the row or column index is out of bounds (as
    /// defined by [`NUM_ROWS`] and [`NUM_COLUMNS`]).
    pub fn set_cursor_position(&mut self, row: u8, col: u8) -> Result<(), Error> {
        if row >= NUM_ROWS || col >= NUM_COLUMNS {
            return Err(Error::InvalidArgument);
        }
        self.transact(&Packet::new(0x0b, &[col, row]))?;
        Ok(())
    }

    /// Set the cursor style.
    pub fn set_cursor_style(&mut self, style: CursorStyle) -> Result<(), Error> {
        self.transact(&Packet::new(0x0c, &[style as u8]))?;
        Ok(())
    }

    /// Set the contrast of the LCD display.
    ///
    /// From the datasheet:
    ///
    /// - 60 = light
    /// - 120 = about right
    /// - 150 = dark
    /// - 151-254 = very dark (may be useful at cold temperatures)
    pub fn set_contrast(&mut self, contrast: u8) -> Result<(), Error> {
        // Clamp to allowed values:
        let contrast = contrast.min(254);
        self.transact(&Packet::new(0x0d, &[contrast]))?;
        Ok(())
    }

    /// Set the backlight brightness of the screen and keypad.
    ///
    /// The maximum brightness value is 100. Values above this will be
    /// interpeted as max brightness.
    ///
    /// From the datasheet:
    ///
    /// - 0 = off
    /// - 1-100 = variable brightness
    pub fn set_backlight(&mut self, screen: u8, keypad: u8) -> Result<(), Error> {
        // Clamp to allowed values:
        let screen = screen.min(100);
        let keypad = keypad.min(100);
        self.transact(&Packet::new(0x0e, &[screen, keypad]))?;
        Ok(())
    }

    /// Configure which key events should be reported by the device.
    ///
    /// Any key code that is present in `press` or `release` will be "enabled"
    /// and will be reported for the respective event. Any key code not present
    /// will likewise be "disabled".
    pub fn configure_key_reporting(&mut self, press: &[Key], release: &[Key]) -> Result<(), Error> {
        let press_mask = press.iter().map(Key::mask).fold(0, |a, b| a | b);
        let release_mask = release.iter().map(Key::mask).fold(0, |a, b| a | b);
        self.transact(&Packet::new(0x17, &[press_mask, release_mask]))?;
        Ok(())
    }

    /// Returns the next report packet, or `None` if there are none available
    /// right now.
    pub fn poll_report(&mut self) -> Result<Option<Report>, Error> {
        if let Some(report) = self.report_buffer.pop_front() {
            Ok(Some(report))
        } else {
            while self.codec.inner().bytes_to_read()? > 0 {
                let packet = self.recv()?;
                if let Some(report) = Report::from_raw(&packet) {
                    return Ok(Some(report));
                }
            }
            Ok(None)
        }
    }

    /// Set the state of an indicator LED.
    ///
    /// The brightness of the red and green components is a value between 0
    /// (off) and 100 (max brightness). A value higher than 100 will be
    /// interpreted as max brightness.
    ///
    /// # Errors
    ///
    /// - `InvalidArgument` - If the LED index is out of bounds (as
    /// defined by [`NUM_LEDS`]).
    pub fn set_led(&mut self, index: u8, red: u8, green: u8) -> Result<(), Error> {
        if index >= NUM_LEDS {
            return Err(Error::InvalidArgument);
        }
        let (red_gpio, green_gpio) = match index {
            0 => (12, 11),
            1 => (10, 9),
            2 => (8, 7),
            3 => (6, 5),
            _ => unreachable!(),
        };
        self.transact(&Packet::new(0x22, &[red_gpio, red]))?;
        self.transact(&Packet::new(0x22, &[green_gpio, green]))?;
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

#[derive(Debug, Clone)]
pub enum Report {
    KeyActivity { key: Key, pressed: bool },
}

impl Report {
    pub fn from_raw(packet: &Packet) -> Option<Self> {
        match packet.packet_type() {
            0x80 => {
                let data = match packet.data().get(0) {
                    Some(&x) => x,
                    None => {
                        log::warn!("not enough bytes for a key activity report");
                        return None;
                    }
                };
                let (key, pressed) = match data {
                    1 => (Key::Up, true),
                    2 => (Key::Down, true),
                    3 => (Key::Left, true),
                    4 => (Key::Right, true),
                    5 => (Key::Enter, true),
                    6 => (Key::Exit, true),
                    7 => (Key::Up, false),
                    8 => (Key::Down, false),
                    9 => (Key::Left, false),
                    10 => (Key::Right, false),
                    11 => (Key::Enter, false),
                    12 => (Key::Exit, false),
                    _ => {
                        log::warn!("unknown key code {:?}", data);
                        return None;
                    }
                };
                Some(Self::KeyActivity { key, pressed })
            }
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Exit,
}

impl Key {
    fn mask(&self) -> u8 {
        match self {
            Self::Up => 0x01,
            Self::Enter => 0x02,
            Self::Exit => 0x04,
            Self::Left => 0x08,
            Self::Right => 0x10,
            Self::Down => 0x20,
        }
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    #[error("serialport: {0}")]
    SerialPort(#[from] serialport::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Read an unexpected/incorrect byte, either because of an incompatibility
    /// or desync.
    #[error("invalid read - potential desync on serial port")]
    InvalidRead,

    /// An argument to a function call had a value that was out of range.
    ///
    /// See the individual functions' documentation for more details about
    /// allowed values of their arguments.
    #[error("invalid value for argument")]
    InvalidArgument,

    /// The device returned an error in its response to our command.
    ///
    /// The cause is usually an error that is out of our control, e.g.
    /// [`Device::save_boot_state`] may return this error if the device doesn't
    /// read back the correct data after saving, which is unlikely but may
    /// eventually happen because of a worn-out flash.
    #[error("Device returned an error response")]
    ReturnedError,
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
            ReadPacketError::InvalidPacket => Self::InvalidRead,
        }
    }
}
