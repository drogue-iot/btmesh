use core::array::TryFromSliceError;
use core::ops::{Add, Sub};

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

impl From<Nid> for u8 {
    fn from(nid: Nid) -> Self {
        nid.0
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

impl From<Aid> for u8 {
    fn from(aid: Aid) -> Self {
        aid.0
    }
}

impl From<u8> for Aid {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

#[derive(Copy, Clone)]
pub enum IvUpdateFlag {
    Normal,
    InProgress,
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct IvIndex(u32);

impl IvIndex {
    pub fn parse(iv_index: &[u8]) -> Result<Self, ParseError> {
        if iv_index.len() == 4 {
            Ok(Self(u32::from_be_bytes([
                iv_index[0],
                iv_index[1],
                iv_index[2],
                iv_index[3],
            ])))
        } else {
            Err(ParseError::InvalidLength)
        }
    }

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

    pub fn accepted_iv_index(&self, ivi: Ivi) -> IvIndex {
        if self.ivi() == ivi {
            *self
        } else {
            *self - 1
        }
    }

    pub fn transmission_iv_index(&self, flag: IvUpdateFlag) -> IvIndex {
        match flag {
            IvUpdateFlag::Normal => *self,
            IvUpdateFlag::InProgress => *self - 1,
        }
    }
}

impl Sub<u8> for IvIndex {
    type Output = Self;

    fn sub(self, rhs: u8) -> Self::Output {
        if self.0 > rhs as u32 {
            Self(self.0 - rhs as u32)
        } else {
            self
        }
    }
}

impl Add<u8> for IvIndex {
    type Output = Self;

    fn add(self, rhs: u8) -> Self::Output {
        Self(self.0 + rhs as u32)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
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

impl From<Ivi> for u8 {
    fn from(ivi: Ivi) -> Self {
        match ivi {
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

#[cfg(test)]
mod test {
    use crate::{IvIndex, IvUpdateFlag, Ivi};

    #[test]
    fn iv_index_zero() {
        let iv_index = IvIndex::parse(&[0x00, 0x00, 0x00, 0x00]).unwrap();
        assert_eq!(Ivi::Zero, iv_index.ivi());

        assert_eq!(iv_index, iv_index.accepted_iv_index(Ivi::Zero));
        // special case, non-valid but don't break.
        assert_eq!(iv_index, iv_index.accepted_iv_index(Ivi::One));

        assert_eq!(
            iv_index,
            iv_index.transmission_iv_index(IvUpdateFlag::Normal)
        );

        // special case, non-valid but don't break.
        assert_eq!(
            iv_index,
            iv_index.transmission_iv_index(IvUpdateFlag::InProgress)
        );
    }

    #[test]
    fn iv_index_non_zero() {
        let iv_index = IvIndex::parse(&[0x00, 0x00, 0x00, 0x03]).unwrap();
        let prev_iv_index = iv_index - 1;

        assert_eq!(iv_index.value(), 3);
        assert_eq!(prev_iv_index.value(), 2);

        assert_eq!(Ivi::One, iv_index.ivi());

        assert_eq!(iv_index, iv_index.accepted_iv_index(Ivi::One));
        assert_eq!(prev_iv_index, iv_index.accepted_iv_index(Ivi::Zero));
    }
}
