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

/// Network key identifier.
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Nid(u8);

impl Nid {
    pub fn new(nid: u8) -> Self {
        Self(nid)
    }

    pub fn parse(nid: u8) -> Result<Nid, ParseError> {
        Ok(Self::new(nid))
    }
}

impl Into<u8> for Nid {
    fn into(self) -> u8 {
        self.0
    }
}

impl From<u8> for Nid {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

/// Application key identifier.
#[derive(Copy, Clone, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Aid(u8);

impl Into<u8> for Aid {
    fn into(self) -> u8 {
        self.0
    }
}

impl From<u8> for Aid {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Ivi {
    Zero,
    One,
}

impl Ivi {
    pub fn parse(ivi: u8) -> Result<Ivi, ParseError> {
        match ivi {
            0 => Ok(Ivi::Zero),
            1 => Ok(Ivi::One),
            _ => Err(ParseError::InvalidValue),
        }
    }
}

impl Into<u8> for Ivi {
    fn into(self) -> u8 {
        match self {
            Ivi::Zero => 0,
            Ivi::One => 1,
        }
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Ttl(u8);

impl Ttl {
    pub fn parse(ttl: u8) -> Result<Ttl, ParseError> {
        Ok(Self(ttl))
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Seq(u32);

impl Seq {
    pub fn parse(seq: u32) -> Result<Seq, ParseError> {
        Ok(Self(seq))
    }
}

#[derive(Copy, Clone)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Ctl {
    Access,
    Control,
}

impl Ctl {
    pub fn parse(ctl: u8) -> Result<Ctl, ParseError> {
        match ctl {
            0 => Ok(Ctl::Access),
            1 => Ok(Ctl::Control),
            _ => Err(ParseError::InvalidValue),
        }
    }

    pub fn netmic_size(&self) -> usize {
        match self {
            Ctl::Access => 4,
            Ctl::Control => 8,
        }
    }
}
