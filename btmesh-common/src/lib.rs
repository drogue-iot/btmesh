use core::array::TryFromSliceError;

pub mod address;
pub mod crypto;
pub mod mic;

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

impl From<TryFromSliceError> for ParseError {
    fn from(_: TryFromSliceError) -> Self {
        Self::InvalidLength
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

impl Aid {
    pub fn parse(akf_aid: u8) -> Result<Option<Self>, ParseError> {
        let akf = akf_aid & 0b01000000 != 0;
        if akf {
            let aid = akf_aid & 0b00111111;
            Ok(Some(Self(aid)))
        } else {
            Ok(None)
        }
    }
}

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

#[derive(Copy, Clone, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IvIndex(u32);

impl IvIndex {
    pub fn to_be_bytes(&self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn ivi(&self) -> Ivi {
        if self.0 & 1 == 1 {
            Ivi::One
        } else {
            Ivi::Zero
        }
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

    pub fn to_be_bytes(&self) -> [u8; 4] {
        self.0.to_be_bytes()
    }
}

#[derive(Copy, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SeqZero(u16);

impl SeqZero {

    pub fn new(seq_zero: u16) -> Self {
        Self(seq_zero)
    }

    pub fn parse(data: u16) -> Result<Self, ParseError> {
        Ok(Self(data))
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
