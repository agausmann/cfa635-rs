use std::io::{Read, Write};

use thiserror::Error;

pub const MAX_DATA_LEN: usize = 22;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReadPacketError {
    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("invalid packet data - might be version mismatch or desync")]
    InvalidPacket,
}

#[derive(Debug, Error)]
pub enum WritePacketError {
    #[error("io error")]
    Io(#[from] std::io::Error),

    #[error("packet has an invalid length")]
    InvalidLength,
}

#[derive(Debug, Clone)]
pub struct Packet {
    packet_type: u8,
    data_len: u8,
    data_array: [u8; MAX_DATA_LEN],
    crc: [u8; 2],
    trusted_crc: bool,
}

impl Packet {
    pub fn new(packet_type: u8, data: &[u8]) -> Self {
        assert!(data.len() <= MAX_DATA_LEN, "packet data is too large");
        let mut packet = Self {
            packet_type,
            data_len: data.len() as u8,
            data_array: [0; MAX_DATA_LEN],
            crc: [0; 2],
            trusted_crc: false,
        };
        packet.data_array[..data.len()].copy_from_slice(data);
        packet.set_crc();
        packet
    }

    pub fn packet_type(&self) -> u8 {
        self.packet_type
    }

    pub fn data(&self) -> &[u8] {
        &self.data_array[..self.data_len as usize]
    }

    /// The byte sequence used to calculate the CRC.
    ///
    /// Exposed as a convenience for other CRC algorithm implementations.
    pub(crate) fn crc_bytes<'a>(&'a self) -> impl Iterator<Item = u8> + 'a {
        [self.packet_type, self.data_len]
            .into_iter()
            .chain(self.data().iter().copied())
    }

    /// Algorithm to calculate the CRC for the given packet data.
    ///
    /// Adapted from [`get_crc` in cfa_linux_examples](https://github.com/crystalfontz/cfa_linux_examples/blob/b07028f2c26c1fb9ae933d07508936bebead4067/include/cf_packet.c)
    pub(crate) fn calculate_crc(&self) -> [u8; 2] {
        #[rustfmt::skip]
        const LOOKUP_TABLE: [u16; 256] = [
            0x0000, 0x1189, 0x2312, 0x329B, 0x4624, 0x57AD, 0x6536, 0x74BF,
            0x8C48, 0x9DC1, 0xAF5A, 0xBED3, 0xCA6C, 0xDBE5, 0xE97E, 0xF8F7,
            0x1081, 0x0108, 0x3393, 0x221A, 0x56A5, 0x472C, 0x75B7, 0x643E,
            0x9CC9, 0x8D40, 0xBFDB, 0xAE52, 0xDAED, 0xCB64, 0xF9FF, 0xE876,
            0x2102, 0x308B, 0x0210, 0x1399, 0x6726, 0x76AF, 0x4434, 0x55BD,
            0xAD4A, 0xBCC3, 0x8E58, 0x9FD1, 0xEB6E, 0xFAE7, 0xC87C, 0xD9F5,
            0x3183, 0x200A, 0x1291, 0x0318, 0x77A7, 0x662E, 0x54B5, 0x453C,
            0xBDCB, 0xAC42, 0x9ED9, 0x8F50, 0xFBEF, 0xEA66, 0xD8FD, 0xC974,
            0x4204, 0x538D, 0x6116, 0x709F, 0x0420, 0x15A9, 0x2732, 0x36BB,
            0xCE4C, 0xDFC5, 0xED5E, 0xFCD7, 0x8868, 0x99E1, 0xAB7A, 0xBAF3,
            0x5285, 0x430C, 0x7197, 0x601E, 0x14A1, 0x0528, 0x37B3, 0x263A,
            0xDECD, 0xCF44, 0xFDDF, 0xEC56, 0x98E9, 0x8960, 0xBBFB, 0xAA72,
            0x6306, 0x728F, 0x4014, 0x519D, 0x2522, 0x34AB, 0x0630, 0x17B9,
            0xEF4E, 0xFEC7, 0xCC5C, 0xDDD5, 0xA96A, 0xB8E3, 0x8A78, 0x9BF1,
            0x7387, 0x620E, 0x5095, 0x411C, 0x35A3, 0x242A, 0x16B1, 0x0738,
            0xFFCF, 0xEE46, 0xDCDD, 0xCD54, 0xB9EB, 0xA862, 0x9AF9, 0x8B70,
            0x8408, 0x9581, 0xA71A, 0xB693, 0xC22C, 0xD3A5, 0xE13E, 0xF0B7,
            0x0840, 0x19C9, 0x2B52, 0x3ADB, 0x4E64, 0x5FED, 0x6D76, 0x7CFF,
            0x9489, 0x8500, 0xB79B, 0xA612, 0xD2AD, 0xC324, 0xF1BF, 0xE036,
            0x18C1, 0x0948, 0x3BD3, 0x2A5A, 0x5EE5, 0x4F6C, 0x7DF7, 0x6C7E,
            0xA50A, 0xB483, 0x8618, 0x9791, 0xE32E, 0xF2A7, 0xC03C, 0xD1B5,
            0x2942, 0x38CB, 0x0A50, 0x1BD9, 0x6F66, 0x7EEF, 0x4C74, 0x5DFD,
            0xB58B, 0xA402, 0x9699, 0x8710, 0xF3AF, 0xE226, 0xD0BD, 0xC134,
            0x39C3, 0x284A, 0x1AD1, 0x0B58, 0x7FE7, 0x6E6E, 0x5CF5, 0x4D7C,
            0xC60C, 0xD785, 0xE51E, 0xF497, 0x8028, 0x91A1, 0xA33A, 0xB2B3,
            0x4A44, 0x5BCD, 0x6956, 0x78DF, 0x0C60, 0x1DE9, 0x2F72, 0x3EFB,
            0xD68D, 0xC704, 0xF59F, 0xE416, 0x90A9, 0x8120, 0xB3BB, 0xA232,
            0x5AC5, 0x4B4C, 0x79D7, 0x685E, 0x1CE1, 0x0D68, 0x3FF3, 0x2E7A,
            0xE70E, 0xF687, 0xC41C, 0xD595, 0xA12A, 0xB0A3, 0x8238, 0x93B1,
            0x6B46, 0x7ACF, 0x4854, 0x59DD, 0x2D62, 0x3CEB, 0x0E70, 0x1FF9,
            0xF78F, 0xE606, 0xD49D, 0xC514, 0xB1AB, 0xA022, 0x92B9, 0x8330,
            0x7BC7, 0x6A4E, 0x58D5, 0x495C, 0x3DE3, 0x2C6A, 0x1EF1, 0x0F78,
        ];
        let mut crc: u16 = 0xffff;
        for b in self.crc_bytes() {
            crc = (crc >> 8) ^ LOOKUP_TABLE[(crc as u8 ^ b) as usize];
        }
        (!crc).to_le_bytes()
    }

    /// Sets the `crc` field using `calculate_crc`.
    pub(crate) fn set_crc(&mut self) {
        self.crc = self.calculate_crc();
        self.trusted_crc = true;
    }

    pub(crate) fn crc(&self) -> [u8; 2] {
        if self.trusted_crc {
            self.crc
        } else {
            self.calculate_crc()
        }
    }

    /// Compares the packet's stored (received) CRC with one calculated from
    /// its data, returning `true` if they are equal.
    pub fn check_crc(&self) -> bool {
        self.calculate_crc() == self.crc
    }
}

impl PartialEq for Packet {
    fn eq(&self, other: &Self) -> bool {
        self.packet_type == other.packet_type && self.data() == other.data()
    }
}

pub struct PacketCodec<T> {
    inner: T,
}

impl<T> PacketCodec<T> {
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> PacketCodec<T>
where
    T: Read,
{
    pub fn read_packet(&mut self) -> Result<Packet, ReadPacketError> {
        let mut packet_type = [0u8; 1];
        self.inner.read_exact(&mut packet_type)?;
        let packet_type = u8::from_le_bytes(packet_type);

        let mut data_len = [0u8; 1];
        self.inner.read_exact(&mut data_len)?;
        let data_len = u8::from_le_bytes(data_len);
        if data_len as usize > MAX_DATA_LEN {
            return Err(ReadPacketError::InvalidPacket);
        }

        let mut data_array = [0u8; MAX_DATA_LEN];
        self.inner
            .read_exact(&mut data_array[..data_len as usize])?;

        let mut crc = [0u8; 2];
        self.inner.read_exact(&mut crc)?;

        let packet = Packet {
            packet_type,
            data_len,
            data_array,
            crc,
            trusted_crc: false,
        };

        Ok(packet)
    }
}

impl<T> PacketCodec<T>
where
    T: Write,
{
    pub fn write_packet(&mut self, packet: &Packet) -> Result<(), WritePacketError> {
        if packet.data_len as usize > MAX_DATA_LEN {
            return Err(WritePacketError::InvalidLength);
        }
        self.inner
            .write_all(&[packet.packet_type, packet.data_len])?;
        self.inner.write_all(packet.data())?;
        self.inner.write_all(&packet.crc())?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), std::io::Error> {
        self.inner.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cross_check_crc() {
        /// Alternative CRC implementation, adapted from Appendix A, Algorithm 2B
        /// of the CFA635 datasheet.
        fn alternate_crc(bytes: impl IntoIterator<Item = u8>) -> [u8; 2] {
            let mut crc: u16 = 0xffff;
            for b in bytes.into_iter() {
                let mut sr = b as u16;
                for _ in 0..8 {
                    if ((crc ^ sr) & 0x01) != 0 {
                        crc >>= 1;
                        crc ^= 0x8408;
                    } else {
                        crc >>= 1;
                    }
                    sr >>= 1;
                }
            }
            (!crc).to_le_bytes()
        }

        let test_packets = [
            Packet::new(10, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            Packet::new(28, &[1, 4, 2, 8, 5, 3, 8, 4, 7, 9, 2, 3]),
        ];
        for (i, packet) in test_packets.iter().enumerate() {
            eprintln!("Test {}...", i);
            assert_eq!(packet.calculate_crc(), alternate_crc(packet.crc_bytes()));
        }
    }

    #[test]
    fn packet_roundtrip() {
        let test_packet = Packet::new(0x00, b"Hello World");
        let mut buffer = Vec::new();
        {
            let mut writer = PacketCodec::new(&mut buffer);
            writer.write_packet(&test_packet).expect("write failed");
        }

        let mut reader = PacketCodec::new(buffer.as_slice());
        let read_packet = reader.read_packet().expect("read failed");
        assert!(read_packet.check_crc());
        assert_eq!(read_packet, test_packet);
    }
}
