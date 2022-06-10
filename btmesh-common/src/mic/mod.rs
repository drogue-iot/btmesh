use crate::mic::SzMic::Bit64;
use crate::ParseError;

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SzMic {
    Bit32,
    Bit64,
}

impl SzMic {
    pub fn len(&self) -> usize {
        match self {
            SzMic::Bit32 => 4,
            SzMic::Bit64 => 8,
        }
    }

    pub fn parse(data: u8) -> Self {
        if data != 0 {
            Self::Bit64
        } else {
            Self::Bit32
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TransMic {
    Bit32(Bit32TransMic),
    Bit64(Bit64TransMic),
}

impl TransMic {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        match data.len() {
            4 => Ok(TransMic::Bit32(Bit32TransMic(data.try_into()?))),
            8 => Ok(TransMic::Bit64(Bit64TransMic(data.try_into()?))),
            _ => Err(ParseError::InvalidLength),
        }
    }

    pub fn szmic(&self) -> SzMic {
        match self {
            TransMic::Bit32(_) => SzMic::Bit32,
            TransMic::Bit64(_) => SzMic::Bit64,
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Bit32TransMic([u8; 4]);

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Bit64TransMic([u8; 6]);
