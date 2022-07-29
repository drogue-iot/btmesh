#![cfg_attr(not(test), no_std)]

use core::array::TryFromSliceError;
use core::ops::{Add, BitAnd, Deref, Sub};
use rand_core::RngCore;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

pub mod address;
pub mod crc;
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

impl From<cmac::crypto_mac::InvalidKeyLength> for ParseError {
    fn from(_: cmac::crypto_mac::InvalidKeyLength) -> Self {
        Self::InvalidLength
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Default, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum IvUpdateFlag {
    #[default]
    Normal,
    InProgress,
}

impl IvUpdateFlag {
    pub fn parse(data: u8) -> Self {
        if data == 0 {
            Self::Normal
        } else {
            Self::InProgress
        }
    }
    pub fn emit(&self, data: &mut u8) {
        if self == &Self::InProgress {
            *data |= 0b00000010;
        }
    }
}

#[derive(Copy, Clone, Hash, PartialEq, Debug, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct KeyRefreshFlag(pub bool);

impl KeyRefreshFlag {
    pub fn parse(data: u8) -> Self {
        Self(data != 0)
    }
    pub fn emit(&self, data: &mut u8) {
        if self.0 {
            *data |= 0b00000001;
        }
    }
}

#[derive(Copy, Clone, Default, PartialEq, Eq, Debug, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct IvIndex(u32);

impl IvIndex {
    pub fn new(iv_index: u32) -> Self {
        Self(iv_index)
    }

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
    pub fn new(ttl: u8) -> Self {
        Self(ttl)
    }

    pub fn parse(ttl: u8) -> Result<Self, ParseError> {
        Ok(Self(ttl))
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

pub struct SeqRolloverError;

#[derive(Default, Copy, Clone, PartialEq, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Seq(u32);

impl Add<u32> for Seq {
    type Output = Result<Seq, SeqRolloverError>;

    fn add(self, rhs: u32) -> Self::Output {
        match self.0.checked_add(rhs) {
            None => Err(SeqRolloverError),
            Some(val) => Ok(Self(val)),
        }
    }
}

impl Seq {
    pub fn new(seq: u32) -> Self {
        Self(seq)
    }

    pub fn parse(seq: u32) -> Result<Seq, ParseError> {
        Ok(Self(seq))
    }

    pub fn to_be_bytes(&self) -> [u8; 4] {
        self.0.to_be_bytes()
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl From<Seq> for SeqZero {
    fn from(seq: Seq) -> Self {
        Self(seq.0 as u16)
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

    pub fn value(&self) -> u16 {
        self.0
    }
}

impl BitAnd<u16> for SeqZero {
    type Output = u16;

    fn bitand(self, rhs: u16) -> Self::Output {
        self.0 & rhs
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

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Uuid([u8; 16]);

impl Uuid {
    pub fn new(uuid: [u8; 16]) -> Self {
        Self(uuid)
    }

    pub fn new_random<R: RngCore>(rng: &mut R) -> Self {
        let mut bytes = [0; 16];
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

impl Deref for Uuid {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct NetworkId([u8; 8]);

impl NetworkId {
    pub fn new(network_id: [u8; 8]) -> Self {
        Self(network_id)
    }
}

impl Deref for NetworkId {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
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
