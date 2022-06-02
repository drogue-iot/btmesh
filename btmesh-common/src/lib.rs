pub mod address;
pub mod crypto;

pub struct InsufficientBuffer;

impl From<()> for InsufficientBuffer {
    fn from(_: ()) -> Self {
        InsufficientBuffer
    }
}

impl From<u8> for InsufficientBuffer {
    fn from(_: u8) -> Self {
        InsufficientBuffer
    }
}

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug)]
pub enum ParseError {
    InvalidPDUFormat,
    InvalidValue,
    InvalidLength,
    InsufficientBuffer,
}

impl From<()> for ParseError {
    fn from(_: ()) -> Self {
        Self::InsufficientBuffer
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SzMic {
    Bit32,
    Bit64,
}

impl SzMic {
    pub fn parse(data: u8) -> Self {
        if data != 0 {
            Self::Bit64
        } else {
            Self::Bit32
        }
    }
}
